use super::*;

#[test]
fn rotated_bootstrap_reconciles_online_without_resetting_rollback_protection() {
    let temp = tempfile::tempdir().unwrap();
    let (mut agent, runtime, old_key) = setup(temp.path());
    agent.acquire(true).unwrap();
    let old_state = runtime.state.lock().unwrap().clone();
    let new_key = SigningKey::from_bytes(&[21; 32]);
    let mut new_public = agent.bootstrap.policy_keys[0].clone();
    new_public.kid = "next".into();
    new_public.x = URL_SAFE_NO_PAD.encode(new_key.verifying_key().as_bytes());
    agent.bootstrap.policy_keys.push(new_public);
    *runtime.reply.lock().unwrap() = None;
    assert!(agent.acquire(false).is_err());
    // Equal sequence and signatures from the wrong key cannot reconcile.
    for (key, sequence) in [(&new_key, 1), (&old_key, 2)] {
        *runtime.reply.lock().unwrap() = Some(envelope(&agent, key, sequence));
        assert!(agent.acquire(true).is_err());
        assert_eq!(*runtime.state.lock().unwrap(), old_state);
    }
    *runtime.reply.lock().unwrap() = Some(envelope(&agent, &new_key, 2));
    assert_eq!(agent.acquire(true).unwrap().snapshot.revision(), "r2");
    *runtime.reply.lock().unwrap() = None;
    assert!(!agent.acquire(false).unwrap().online);
    // Removing the old key is another bootstrap transition, still monotonic.
    agent.bootstrap.policy_keys.remove(0);
    *runtime.reply.lock().unwrap() = Some(envelope(&agent, &new_key, 1));
    assert!(agent.acquire(true).is_err());
    *runtime.reply.lock().unwrap() = Some(envelope(&agent, &new_key, 3));
    assert_eq!(agent.acquire(true).unwrap().snapshot.revision(), "r3");
}

#[test]
fn elapsed_refresh_and_store_time_cannot_extend_policy_deadlines() {
    use std::sync::atomic::Ordering::SeqCst;
    for (online, during_store) in [(true, false), (false, false), (true, true), (false, true)] {
        let temp = tempfile::tempdir().unwrap();
        let (agent, runtime, _) = setup(temp.path());
        agent.acquire(true).unwrap();
        let lifetime = if online { 172800 } else { 86400 };
        *runtime.time.lock().unwrap() += lifetime - 5;
        if !online {
            *runtime.reply.lock().unwrap() = None;
        }
        if during_store {
            runtime.store_elapsed.store(20, SeqCst);
        } else {
            runtime.refresh_elapsed.store(20, SeqCst);
        }
        assert!(
            agent.acquire(online).is_err(),
            "online={online}, store={during_store}"
        );
    }
    let temp = tempfile::tempdir().unwrap();
    let (agent, runtime, _) = setup(temp.path());
    agent.acquire(true).unwrap();
    runtime.store_elapsed.store(86400, SeqCst);
    // Even the no-network fast path must recheck time after its store write.
    assert!(agent.acquire(false).is_err());
    let temp = tempfile::tempdir().unwrap();
    let (agent, runtime, _) = setup(temp.path());
    runtime.refresh_elapsed.store(-1, SeqCst);
    assert!(agent
        .acquire(true)
        .err()
        .unwrap()
        .to_string()
        .contains("POLICY_CLOCK_UNCERTAIN"));
}

#[test]
fn denial_remains_sticky_when_clock_moves_during_transport() {
    use std::sync::atomic::Ordering::SeqCst;
    let temp = tempfile::tempdir().unwrap();
    let (agent, runtime, _) = setup(temp.path());
    agent.acquire(true).unwrap();
    *runtime.denied.lock().unwrap() = true;
    runtime.refresh_elapsed.store(-1, SeqCst);
    assert!(agent.acquire(true).is_err());
    assert!(agent.state().unwrap().unwrap().denied);
    runtime.refresh_elapsed.store(0, SeqCst);
    *runtime.time.lock().unwrap() += 1;
    *runtime.denied.lock().unwrap() = false;
    *runtime.reply.lock().unwrap() = None;
    assert!(agent.acquire(false).is_err());
}
