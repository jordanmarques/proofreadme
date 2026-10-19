//! The single instance: a second launch asks the already running agent to show its settings, then
//! ends.
//!
//! The two processes talk through a local socket (`interprocess` crate): a Unix domain socket
//! placed in a folder of the user on macOS, a named pipe on Windows. Holding the socket is being
//! the agent; managing to connect to it is finding another one.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use interprocess::local_socket::traits::{ListenerExt, Stream as _};
use interprocess::local_socket::{GenericFilePath, GenericNamespaced, Listener, ListenerOptions, Name, Stream, ToFsName, ToNsName};

use crate::session::{Request, RequestSender};

/// The message by which a second launch asks for the window to be shown.
const SHOW: &str = "show";

/// What a launch learns when announcing itself.
pub enum Instance {
    /// No agent was running: this process becomes it, and must listen to the next ones.
    First(InstanceListener),
    /// An agent is already running, and the request to show its window was handed to it: this
    /// process only has to end.
    AlreadyRunning,
}

/// The socket held by the agent, waiting for the next launches.
pub struct InstanceListener {
    socket: Listener,
}

/// Announces itself as an instance of the agent identified by `identifier`, whose socket lives in
/// `folder`. On Windows, where the named pipe has no folder, `folder` only serves to make its name
/// specific to the user.
///
/// # Errors
///
/// The input-output error if the socket can be neither joined nor created. The caller can then go
/// on without a single instance guarantee.
pub fn announce(folder: &Path, identifier: &str) -> io::Result<Instance> {
    if request_show(name(folder, identifier)?).is_ok() {
        return Ok(Instance::AlreadyRunning);
    }
    // Nobody responds. The file of an agent that died abruptly may remain: it is replaced.
    std::fs::create_dir_all(folder)?;
    let socket = ListenerOptions::new().name(name(folder, identifier)?).try_overwrite(true).create_sync()?;
    Ok(Instance::First(InstanceListener { socket }))
}

impl InstanceListener {
    /// Listens to the next launches on a separate thread: each one brings back the settings
    /// window of the agent ([`Request::OpenSettings`], user story 47), sent through `requests`.
    ///
    /// # Errors
    ///
    /// The input-output error if the thread cannot be created.
    pub fn listen(self, requests: RequestSender) -> io::Result<()> {
        let show: Arc<dyn Fn() + Send + Sync> = Arc::new(move || requests.send(Request::OpenSettings));
        let reads = Arc::new(AtomicUsize::new(0));
        thread::Builder::new().name("proofreadme-instance".to_owned()).spawn(move || {
            for connection in self.socket.incoming().flatten() {
                // Each client is read on its own thread: one that stays silent does not hold up
                // the listening to the next ones. Beyond a few reads in progress, the client is
                // turned away rather than piling up threads.
                if reads.fetch_add(1, Ordering::SeqCst) >= MAX_READS {
                    reads.fetch_sub(1, Ordering::SeqCst);
                    continue;
                }
                let (show, in_progress) = (Arc::clone(&show), ReadInProgress(Arc::clone(&reads)));
                // If the OS refuses the thread, the closure is destroyed and the read counted down.
                let _ = thread::Builder::new().name("proofreadme-instance-read".to_owned()).spawn(move || {
                    let _in_progress = in_progress;
                    if read_message(connection).is_some_and(|message| message.trim_end() == SHOW) {
                        show();
                    }
                });
            }
        })?;
        Ok(())
    }
}

/// The number of clients read at the same time, at most.
const MAX_READS: usize = 4;

/// The length beyond which a message is no longer read: no known message comes close to it.
const MAX_LENGTH: u64 = 64;

/// The time left to a client to send its message, where the OS can bound a read (not on the named
/// pipes of Windows, where a silent client holds its read until it leaves).
const MESSAGE_DELAY: Duration = Duration::from_secs(2);

/// A read in progress, counted down when it ends, including if its thread panics or does not
/// start.
struct ReadInProgress(Arc<AtomicUsize>);

impl Drop for ReadInProgress {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The first line sent by a client, bounded in length and, if the OS allows it, in duration.
fn read_message(connection: Stream) -> Option<String> {
    // With no delay possible, the length bound and the separate thread still protect the listening.
    let _ = connection.set_recv_timeout(Some(MESSAGE_DELAY));
    let mut message = String::new();
    BufReader::new(connection).take(MAX_LENGTH).read_line(&mut message).ok()?;
    Some(message)
}

/// Hands the agent that holds the socket the request to show its window. Fails if nobody is
/// listening to it.
fn request_show(name: Name<'_>) -> io::Result<()> {
    let mut connection = Stream::connect(name)?;
    connection.write_all(SHOW.as_bytes())?;
    connection.write_all(b"\n")?;
    connection.flush()
}

/// The name of the socket: a named pipe on Windows, a `<identifier>.sock` file elsewhere.
fn name(folder: &Path, identifier: &str) -> io::Result<Name<'static>> {
    if cfg!(windows) {
        pipe_name(folder, identifier).to_ns_name::<GenericNamespaced>()
    } else {
        folder.join(format!("{identifier}.sock")).to_fs_name::<GenericFilePath>()
    }
}

/// The name of the Windows named pipe. Named pipes are shared by the whole machine: the name
/// therefore carries the identifier of the application, and a fingerprint of the folder of the
/// user, so that two sessions open on the same machine each have their agent.
fn pipe_name(folder: &Path, identifier: &str) -> String {
    format!("{}.{identifier}.{:016x}", crate::IDENTIFIER, fingerprint(folder.as_os_str().as_encoded_bytes()))
}

/// A stable fingerprint of a sequence of bytes (FNV-1a, 64 bits). Written here rather than taken
/// from the standard library, whose hashing can change from one version to the next: an already
/// running agent and a more recent executable must compute the same name.
fn fingerprint(bytes: &[u8]) -> u64 {
    const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(BASIS, |fingerprint, &byte| (fingerprint ^ u64::from(byte)).wrapping_mul(PRIME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_named_pipe_is_specific_to_the_application_and_to_the_folder_of_the_user() {
        let (alice, bruno) = (Path::new(r"C:\Users\alice\AppData\Roaming\ProofReadMe"), Path::new(r"C:\Users\bruno\AppData\Roaming\ProofReadMe"));

        assert!(pipe_name(alice, "agent").starts_with("io.github.jordanmarques.proofreadme.agent."), "{}", pipe_name(alice, "agent"));
        assert_ne!(pipe_name(alice, "agent"), pipe_name(bruno, "agent"), "two users do not have the same pipe");
        assert_eq!(pipe_name(alice, "agent"), pipe_name(alice, "agent"), "two launches of the same user find each other");
    }

    #[test]
    fn the_fingerprint_is_that_of_fnv_1a() {
        // Reference values of the algorithm (public FNV test set).
        assert_eq!(fingerprint(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fingerprint(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fingerprint(b"foobar"), 0x8594_4171_f739_67e8);
    }
}
