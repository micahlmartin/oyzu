use super::*;
use crate::tools::{native_tool_worker_channel, ToolWorkerChannel};
use std::io::Write;

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

#[test]
fn absolute_deadlines_interrupt_live_peer_reads_and_writes() {
    for read in [true, false] {
        let (endpoint, mut peer) = native_tool_worker_channel().unwrap();
        let mut io = NativeToolWorkerIo::new(endpoint).unwrap();
        let error = if read {
            peer.write_all(&[0, 0]).unwrap();
            io.start_receive().unwrap();
            io.wait_receive_frame(Instant::now() + Duration::from_millis(20))
                .err()
                .unwrap()
        } else {
            io.start_send(
                serde_json::to_vec(&serde_json::json!({
                    "data": "x".repeat(2 * 1024 * 1024)
                }))
                .unwrap(),
            )
            .unwrap();
            io.wait_send(Instant::now() + Duration::from_millis(20))
                .unwrap_err()
        };
        assert_eq!(error.to_string(), "TOOL_WORKER_IO_DEADLINE_EXCEEDED");
        assert!(io.start_receive().is_err());
        assert!(io.start_send(b"{}".to_vec()).is_err());
        io.shutdown(deadline()).unwrap();
        assert!(io.read.is_none() && io.write.is_none());
        drop(peer);
    }
}

#[test]
fn deadline_waits_deliver_frames_but_reject_already_completed_late_results() {
    let (endpoint, peer) = native_tool_worker_channel().unwrap();
    let mut io = NativeToolWorkerIo::new(endpoint).unwrap();
    let mut wire = ToolWorkerChannel::new(peer);
    io.start_send(b"{}".to_vec()).unwrap();
    io.wait_send(deadline()).unwrap();
    assert_eq!(wire.receive().unwrap(), serde_json::json!({}));
    wire.send(br#"{ "exact": true }"#).unwrap();
    io.start_receive().unwrap();
    assert_eq!(
        io.wait_receive_frame(deadline()).unwrap().bytes(),
        br#"{ "exact": true }"#
    );
    wire.send(b"{}").unwrap();
    io.start_receive().unwrap();
    let watchdog = deadline();
    while !io.read.as_ref().unwrap().is_finished() {
        assert!(Instant::now() < watchdog);
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        io.wait_receive_frame(Instant::now())
            .err()
            .unwrap()
            .to_string(),
        "TOOL_WORKER_IO_DEADLINE_EXCEEDED"
    );
    assert!(io.try_receive().is_err());
    io.shutdown(deadline()).unwrap();
}

fn receive(io: &mut NativeToolWorkerIo) -> Value {
    let end = deadline();
    loop {
        if let Some(value) = io.try_receive().unwrap() {
            return value;
        }
        assert!(Instant::now() < end, "receive completion timed out");
        thread::sleep(Duration::from_millis(1));
    }
}

fn sent(io: &mut NativeToolWorkerIo) {
    let end = deadline();
    while !io.try_send().unwrap() {
        assert!(Instant::now() < end, "send completion timed out");
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn shutdown_interrupts_both_directions_with_live_peer() {
    let (endpoint, mut peer) = native_tool_worker_channel().unwrap();
    // Keep the peer alive through shutdown. Neither a complete frame nor EOF
    // can release the read, and the peer never drains the backpressured write.
    peer.write_all(&[0, 0]).unwrap();
    let mut io = NativeToolWorkerIo::new(endpoint).unwrap();
    io.start_receive().unwrap();
    io.start_send(
        serde_json::to_vec(&serde_json::json!({
            "data": "x".repeat(2 * 1024 * 1024)
        }))
        .unwrap(),
    )
    .unwrap();
    thread::sleep(Duration::from_millis(50));
    assert!(io.try_receive().unwrap().is_none());
    assert!(!io.try_send().unwrap());
    io.shutdown(deadline()).unwrap();
    assert!(
        io.read.is_none() && io.write.is_none(),
        "both threads joined"
    );
    assert!(io.try_receive().is_err());
    assert!(io.try_send().is_err());
    assert!(io.start_receive().is_err());
    assert!(io.start_send(b"{}".to_vec()).is_err());
    io.shutdown(deadline()).unwrap();
    drop(peer);
}

#[test]
fn full_duplex_frames_and_later_cancel_do_not_block_each_other() {
    let (endpoint, peer) = native_tool_worker_channel().unwrap();
    let mut io = NativeToolWorkerIo::new(endpoint).unwrap();
    let worker = thread::spawn(move || {
        let mut wire = ToolWorkerChannel::new(peer);
        assert_eq!(
            wire.receive().unwrap(),
            serde_json::json!({"request": true})
        );
        assert_eq!(wire.receive().unwrap(), serde_json::json!({"cancel": true}));
        wire.send(br#"{"cancelled":true}"#).unwrap();
    });
    io.start_receive().unwrap();
    io.start_send(br#"{"request":true}"#.to_vec()).unwrap();
    sent(&mut io);
    assert!(io.try_receive().unwrap().is_none());
    io.start_send(br#"{"cancel":true}"#.to_vec()).unwrap();
    sent(&mut io);
    assert_eq!(receive(&mut io), serde_json::json!({"cancelled": true}));
    worker.join().unwrap();
    io.shutdown(deadline()).unwrap();
}

#[test]
fn cancelling_one_channel_leaves_another_operational() {
    let (endpoint, retained_peer) = native_tool_worker_channel().unwrap();
    let mut cancelled = NativeToolWorkerIo::new(endpoint).unwrap();
    cancelled.start_receive().unwrap();
    let (endpoint, peer) = native_tool_worker_channel().unwrap();
    let mut independent = NativeToolWorkerIo::new(endpoint).unwrap();
    independent.start_receive().unwrap();
    cancelled.shutdown(deadline()).unwrap();
    let mut wire = ToolWorkerChannel::new(peer);
    wire.send(br#"{"independent":true}"#).unwrap();
    assert_eq!(
        receive(&mut independent),
        serde_json::json!({"independent": true})
    );
    independent.shutdown(deadline()).unwrap();
    drop(retained_peer);
}

#[test]
fn shutdown_discards_completed_racing_result_and_drop_joins_pending_read() {
    let (endpoint, peer) = native_tool_worker_channel().unwrap();
    let mut io = NativeToolWorkerIo::new(endpoint).unwrap();
    io.start_receive().unwrap();
    let mut wire = ToolWorkerChannel::new(peer);
    wire.send(br#"{"ok":true}"#).unwrap();
    let end = deadline();
    while !io.read.as_ref().unwrap().is_finished() {
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(1));
    }
    io.shutdown(deadline()).unwrap();
    assert!(io.try_receive().is_err());
    let (endpoint, retained_peer) = native_tool_worker_channel().unwrap();
    let mut io = NativeToolWorkerIo::new(endpoint).unwrap();
    io.start_receive().unwrap();
    let start = Instant::now();
    drop(io);
    assert!(start.elapsed() < Duration::from_secs(10));
    drop(retained_peer);
}

#[test]
fn pending_and_invalid_operations_cannot_start_extra_threads() {
    let (endpoint, peer) = native_tool_worker_channel().unwrap();
    let mut io = NativeToolWorkerIo::new(endpoint).unwrap();
    assert!(io.try_receive().is_err());
    assert!(io.try_send().is_err());
    io.start_receive().unwrap();
    assert_eq!(
        io.start_receive().unwrap_err().to_string(),
        "TOOL_WORKER_IO_PENDING"
    );
    assert!(io.start_send(b"{bad".to_vec()).is_err());
    assert!(io.start_send(b"{}".to_vec()).is_err());
    io.shutdown(deadline()).unwrap();
    drop(peer);
}

#[test]
fn cleanup_timeout_retains_thread_until_completion_can_be_confirmed() {
    let (endpoint, peer) = native_tool_worker_channel().unwrap();
    let mut io = NativeToolWorkerIo::new(endpoint).unwrap();
    let receiver = io.receiver.take().unwrap();
    let (release, wait) = std::sync::mpsc::channel();
    // Simulate cancellation not yet acknowledged by the OS. This dedicated
    // thread cannot finish until released; no scheduling race controls expiry.
    io.read = Some(thread::spawn(move || {
        wait.recv().unwrap();
        (receiver, Err(anyhow::anyhow!("cancelled")))
    }));
    let error = io.shutdown(Instant::now()).unwrap_err();
    assert_eq!(error.to_string(), "TOOL_WORKER_IO_SHUTDOWN_TIMEOUT");
    assert!(io.read.is_some(), "timeout must not detach the thread");
    assert!(
        io.try_receive().is_err(),
        "expired cleanup cannot expose output"
    );
    release.send(()).unwrap();
    io.shutdown(deadline()).unwrap();
    assert!(io.read.is_none());
    drop(peer);
}
