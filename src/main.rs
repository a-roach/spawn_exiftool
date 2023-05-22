use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::sync::mpsc;

fn main() {
    // Set up a channel to let our threads talk to each other
    // We will copy the transmitter so both stderr and stdout have transmitters, 
    // but we will have only one receiver in our main thread.
    let (stdout_transmitter, rx) = mpsc::channel();
    let stderr_transmitter=stdout_transmitter.clone();

    // Create a new process and spawn ExifTool into that process
    //  • I have ExifTool in my search path, so do not need to specify its absolute path
    //  • We are going to run ExifTool in "stay open" mode and send commands to it from stdin.
    //    Because of this, we need to steal stdin for input, stdout to capture the output, and
    //    stderr so we can monitor for errors. Parsing stderr and stdout will ultimately happen
    //    in parallel threads so we don't lock up anything.
    //
    let mut exiftool_process = Command::new("ExifTool.exe")
        .args(["-stay_open", "true", "-@", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Take ownership of stdout so we can pass to a separate thread.
    let exiftool_stdout = exiftool_process
        .stdout
        .take()
        .expect("Could not take stdout");

    // Take ownership of stdin so we can pass to a separate thread.
    let exiftool_stderr = exiftool_process
        .stderr
        .take()
        .expect("Could not take stderr");

    // Grab stdin so we can pipe commands to ExifTool
    let exiftool_stdin = exiftool_process.stdin.as_mut().unwrap();

    // Create a separate thread to loop over stdout
    // We are not going to join the tread or anything like that, so we don't need the return
    // value, but if we did want to do something with it, we could.
    let _stdout_thread = thread::spawn(move || {
        let stdout_lines = BufReader::new(exiftool_stdout).lines();

        for line in stdout_lines {
            let line = line.unwrap();
            
            // Check to see if our processing has finished, if it has we will send a message to our main thread.
            if line=="{ready}" {
                stdout_transmitter.send(line).unwrap();
            }
            else {
                // Do some processing out the output from our command. In this case we will just print it.
                println!("->{}", line); 
            }
        }
    });

    // Create a separate thread to loop over stderr
    // Anything which comes through stderr will just be sent back to our calling thread, and will trip an error.
    let _stderr_thread = thread::spawn(move || {
        let stderr_lines = BufReader::new(exiftool_stderr).lines();
        for line in stderr_lines {
            let line = line.unwrap();
            stderr_transmitter.send(line).unwrap();
        }
    });

    // Send a command through to ExifTool using its stdin pipe, then wait for a response from the thread. 
    // If successful we should get "{ready}", in which case we could send our next command if we wanted to.
    // We have to send as "bytes" rather than rust's default UTF16.
    exiftool_stdin.write(b"-ver\n-execute\n").unwrap(); // Boring command I know 🙄
    let received = rx.recv().unwrap(); // wait for the command to finish
    if received=="{ready}" {
        println!("Command seemed to run successfully 😊, so in theory we could fire off another now because ExifTool is ready and waiting, but we will exit instead.")
    } else {
        println!("Well, that was not expected!🤔")
    }
}
