use mg_protocol::proto;

use super::*;
use crate::ids::FriendLinkCode;
use crate::push::PushTarget;
use crate::social::AccountId;

const DAY_MS: i64 = 24 * 3600 * 1000;

fn account(store: &Store, digit: char, name: &str) -> AccountId {
    let id = AccountId::parse(&format!("g_{}", digit.to_string().repeat(32))).unwrap();
    store
        .remember_account(
            &AccountRecord {
                id: id.clone(),
                display_name: name.to_owned(),
                avatar_url: String::new(),
            },
            0,
        )
        .unwrap();
    id
}

fn names(accounts: &[AccountRecord]) -> Vec<&str> {
    accounts
        .iter()
        .map(|account| account.display_name.as_str())
        .collect()
}

fn link(store: &Store, owner: &AccountId, kind: FriendLinkKind) -> FriendLinkCode {
    let code = FriendLinkCode::random();
    store
        .create_friend_link(
            owner,
            &FriendLinkRecord {
                code: code.clone(),
                kind,
                expires_ms: 7 * DAY_MS,
            },
        )
        .unwrap();
    code
}

fn web_push_target(endpoint: &str) -> PushTarget {
    PushTarget::parse(Some(proto::PushTarget {
        target: Some(proto::push_target::Target::WebPush(
            proto::WebPushSubscription {
                endpoint: endpoint.to_owned(),
                p256dh: "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4".to_owned(),
                auth: "BTBZMqHH6r4Tts7J_aSIgg".to_owned(),
            },
        )),
    }))
    .unwrap()
}

#[test]
fn a_request_waits_for_an_answer_and_accepting_makes_friends() {
    let store = Store::in_memory().unwrap();
    let ada = account(&store, 'a', "Ada");
    let bob = account(&store, 'b', "Bob");

    let sent = store.send_friend_request(&ada, &bob, 1).unwrap();
    let bob_sees = store.friend_graph(&bob, 1).unwrap();
    let answered = store
        .answer_friend_request(&bob, &ada, FriendRequestDecision::Accept, 2)
        .unwrap();

    assert_eq!(sent, FriendRequestOutcome::Sent);
    assert_eq!(names(&bob_sees.incoming_requests), ["Ada"]);
    assert!(answered);
    assert_eq!(
        names(&store.friend_graph(&ada, 2).unwrap().friends),
        ["Bob"]
    );
    assert!(
        store
            .friend_graph(&bob, 2)
            .unwrap()
            .incoming_requests
            .is_empty()
    );
}

#[test]
fn asking_someone_who_already_asked_makes_friends_at_once() {
    let store = Store::in_memory().unwrap();
    let ada = account(&store, 'a', "Ada");
    let bob = account(&store, 'b', "Bob");

    store.send_friend_request(&ada, &bob, 1).unwrap();
    let outcome = store.send_friend_request(&bob, &ada, 2).unwrap();

    assert_eq!(outcome, FriendRequestOutcome::BecameFriends);
    assert!(store.are_friends(&ada, &bob).unwrap());
    assert_eq!(
        store.send_friend_request(&ada, &bob, 3).unwrap(),
        FriendRequestOutcome::AlreadyFriends
    );
}

#[test]
fn a_declined_request_leaves_no_friendship() {
    let store = Store::in_memory().unwrap();
    let ada = account(&store, 'a', "Ada");
    let bob = account(&store, 'b', "Bob");
    store.send_friend_request(&ada, &bob, 1).unwrap();

    store
        .answer_friend_request(&bob, &ada, FriendRequestDecision::Decline, 2)
        .unwrap();

    assert!(!store.are_friends(&ada, &bob).unwrap());
    assert!(
        store
            .friend_graph(&ada, 2)
            .unwrap()
            .outgoing_requests
            .is_empty()
    );
}

#[test]
fn a_single_use_link_befriends_only_the_first_who_opens_it() {
    let store = Store::in_memory().unwrap();
    let ada = account(&store, 'a', "Ada");
    let bob = account(&store, 'b', "Bob");
    let cid = account(&store, 'c', "Cid");
    let code = link(&store, &ada, FriendLinkKind::SingleUse);

    let first = store.use_friend_link(&code, &bob, 1).unwrap();
    let second = store.use_friend_link(&code, &cid, 2).unwrap();

    assert_eq!(first, Ok(UsedFriendLink { owner: ada.clone() }));
    assert_eq!(second, Err(FriendLinkRefusal::Invalid));
    assert_eq!(
        names(&store.friend_graph(&ada, 2).unwrap().friends),
        ["Bob"]
    );
}

#[test]
fn a_multi_use_link_befriends_everyone_until_it_expires() {
    let store = Store::in_memory().unwrap();
    let ada = account(&store, 'a', "Ada");
    let bob = account(&store, 'b', "Bob");
    let cid = account(&store, 'c', "Cid");
    let dan = account(&store, 'd', "Dan");
    let code = link(&store, &ada, FriendLinkKind::MultiUse);

    store.use_friend_link(&code, &bob, 1).unwrap().unwrap();
    store.use_friend_link(&code, &cid, 2).unwrap().unwrap();
    let late = store.use_friend_link(&code, &dan, 7 * DAY_MS).unwrap();

    assert_eq!(late, Err(FriendLinkRefusal::Invalid));
    assert_eq!(
        names(&store.friend_graph(&ada, 2).unwrap().friends),
        ["Bob", "Cid"]
    );
}

#[test]
fn opening_ones_own_link_does_nothing() {
    let store = Store::in_memory().unwrap();
    let ada = account(&store, 'a', "Ada");
    let code = link(&store, &ada, FriendLinkKind::SingleUse);

    let used = store.use_friend_link(&code, &ada, 1).unwrap();

    assert_eq!(used, Err(FriendLinkRefusal::OwnLink));
    assert_eq!(store.friend_graph(&ada, 1).unwrap().links.len(), 1);
}

#[test]
fn a_revoked_link_no_longer_works() {
    let store = Store::in_memory().unwrap();
    let ada = account(&store, 'a', "Ada");
    let bob = account(&store, 'b', "Bob");
    let code = link(&store, &ada, FriendLinkKind::MultiUse);

    store.revoke_friend_link(&ada, &code).unwrap();

    assert_eq!(
        store.use_friend_link(&code, &bob, 1).unwrap(),
        Err(FriendLinkRefusal::Invalid)
    );
}

#[test]
fn removing_a_friend_ends_the_friendship_for_both() {
    let store = Store::in_memory().unwrap();
    let ada = account(&store, 'a', "Ada");
    let bob = account(&store, 'b', "Bob");
    store.send_friend_request(&ada, &bob, 1).unwrap();
    store.send_friend_request(&bob, &ada, 2).unwrap();

    store.remove_friend(&bob, &ada).unwrap();

    assert!(store.friend_graph(&ada, 3).unwrap().friends.is_empty());
    assert!(store.friend_graph(&bob, 3).unwrap().friends.is_empty());
}

#[test]
fn a_device_signing_in_to_another_account_moves_its_push_target() {
    let store = Store::in_memory().unwrap();
    let ada = account(&store, 'a', "Ada");
    let bob = account(&store, 'b', "Bob");
    let tablet = web_push_target("https://web.push.apple.com/tablet");

    store.save_push_target(&ada, &tablet, 1).unwrap();
    store.save_push_target(&bob, &tablet, 2).unwrap();

    assert!(store.push_targets(&ada).unwrap().is_empty());
    assert_eq!(store.push_targets(&bob).unwrap(), [tablet]);
}

#[test]
fn an_account_keeps_its_ten_latest_push_targets() {
    let store = Store::in_memory().unwrap();
    let ada = account(&store, 'a', "Ada");

    for device in 0..12 {
        let target = web_push_target(&format!("https://web.push.apple.com/{device}"));
        store.save_push_target(&ada, &target, device).unwrap();
    }

    let kept = store.push_targets(&ada).unwrap();
    assert_eq!(kept.len(), 10);
    assert!(!kept.contains(&web_push_target("https://web.push.apple.com/0")));
    assert!(kept.contains(&web_push_target("https://web.push.apple.com/11")));
}

/// A file in the temp folder that is removed with its WAL files when the test ends.
struct TempDatabase(std::path::PathBuf);

impl TempDatabase {
    fn new() -> Self {
        let name = format!("mg-store-test-{}.db", FriendLinkCode::random());
        Self(std::env::temp_dir().join(name))
    }
}

impl Drop for TempDatabase {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let mut path = self.0.clone().into_os_string();
            path.push(suffix);
            // Best effort: a leftover temp file harms nothing.
            if let Err(error) = std::fs::remove_file(&path) {
                tracing::debug!(%error, "temp database not removed");
            }
        }
    }
}

#[test]
fn friends_survive_a_restart() {
    let file = TempDatabase::new();
    {
        let store = Store::open(&file.0).unwrap();
        let ada = account(&store, 'a', "Ada");
        let bob = account(&store, 'b', "Bob");
        store.send_friend_request(&ada, &bob, 1).unwrap();
        store.send_friend_request(&bob, &ada, 2).unwrap();
    }

    let reopened = Store::open(&file.0).unwrap();

    let ada = AccountId::parse(&format!("g_{}", "a".repeat(32))).unwrap();
    assert_eq!(
        names(&reopened.friend_graph(&ada, 3).unwrap().friends),
        ["Bob"]
    );
}

#[test]
fn a_database_written_by_a_newer_server_is_refused() {
    let file = TempDatabase::new();
    {
        let connection = rusqlite::Connection::open(&file.0).unwrap();
        connection
            .pragma_update(
                None,
                "user_version",
                super::migrations::latest_version() as i64 + 1,
            )
            .unwrap();
    }

    let opened = Store::open(&file.0);

    assert!(matches!(opened, Err(StoreError::NewerThanServer { .. })));
}
