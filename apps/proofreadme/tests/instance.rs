//! The single instance, on a real local socket placed in a temporary folder: nothing is shared
//! with an agent really running on the machine.

use std::sync::mpsc;
use std::time::Duration;

use proofreadme::{Instance, announce};

/// An identifier specific to this test: on Windows the named pipe is global to the machine.
fn identifier(test: &str) -> String {
    format!("proofreadme-test-{}-{test}", std::process::id())
}

#[test]
fn the_second_launch_asks_the_first_to_show_its_window() {
    let folder = tempfile::tempdir().unwrap();
    let identifier = identifier("second");
    let (shows, received) = mpsc::channel();
    let Instance::First(listener) = announce(folder.path(), &identifier).unwrap() else { panic!("the first launch becomes the agent") };
    listener.listen(move || shows.send(()).unwrap()).unwrap();

    let second = announce(folder.path(), &identifier).unwrap();

    assert!(matches!(second, Instance::AlreadyRunning), "the second launch only has to end");
    received.recv_timeout(Duration::from_secs(5)).expect("the first received the request to show its window");
}

#[cfg(unix)]
#[test]
fn a_silent_or_talkative_client_does_not_hold_up_the_listening_to_the_next_launches() {
    use std::io::Write as _;
    use std::os::unix::net::UnixStream;

    let folder = tempfile::tempdir().unwrap();
    let identifier = identifier("silent");
    let (shows, received) = mpsc::channel();
    let Instance::First(listener) = announce(folder.path(), &identifier).unwrap() else { panic!("the first launch becomes the agent") };
    listener.listen(move || shows.send(()).unwrap()).unwrap();
    let socket = folder.path().join(format!("{identifier}.sock"));
    // One client connects and says nothing; another talks without ever finishing its line.
    let _silent = UnixStream::connect(&socket).unwrap();
    let mut talkative = UnixStream::connect(&socket).unwrap();
    talkative.write_all(&[b'x'; 4096]).unwrap();

    let next = announce(folder.path(), &identifier).unwrap();

    assert!(matches!(next, Instance::AlreadyRunning));
    received.recv_timeout(Duration::from_secs(1)).expect("the request of the next launch is received despite the two other clients");
}

#[cfg(unix)]
#[test]
fn the_socket_left_by_a_dead_agent_does_not_block_the_next_launch() {
    let folder = tempfile::tempdir().unwrap();
    let identifier = identifier("dead");
    // An agent killed abruptly leaves the file of its socket, which nobody listens to any more.
    drop(std::os::unix::net::UnixListener::bind(folder.path().join(format!("{identifier}.sock"))).unwrap());

    let next = announce(folder.path(), &identifier).unwrap();

    assert!(matches!(next, Instance::First(_)));
}
