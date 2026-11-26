//! What TS-3 adds to the server: the tokens an administrator makes itself (the invitation's, derived from the secret
//! after the `#`), used once and said used, the reuse told to the administrators; the roles kept in step with the
//! roster; the recovery token; the pairing relay of the short code; the page of an invitation link. Each test once on
//! SQLite, and once on PostgreSQL when `ESCOUADE_TEST_POSTGRES_URL` is set.

#[macro_use]
mod common;

use common::*;
use escouade_team_server::api::wire::*;
use escouade_team_server::db::Role;
use escouade_team_server::util::token_hash;
use reqwest::{Method, StatusCode};

both!(
    a_token_made_by_the_administrator_enrols_once_and_its_reuse_is_told,
    the_roles_follow_the_administrator_and_one_stays,
    the_recovery_token_enrols_an_administrator_of_the_team,
    the_pairing_relay_takes_one_try,
);

async fn register(admin: &Device, token: &str, expires_in: Option<u64>) -> Answer<Registered> {
    admin
        .client
        .post(
            "/v1/devices/enrollments",
            &EnrollmentRequest {
                expires_in,
                token_hash: Some(token_hash(token).to_vec()),
            },
        )
        .await
}

async fn a_token_made_by_the_administrator_enrols_once_and_its_reuse_is_told(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let token = "derived-from-the-secret-after-the-hash-0001";
    let r = register(&admin, token, Some(72 * 3600)).await.ok();
    assert_eq!(r.expires_in, 72 * 3600);
    // The same hash twice: refused, without saying whose it is.
    assert_eq!(
        register(&admin, token, None).await.refused().0,
        StatusCode::CONFLICT
    );
    // A hash of the wrong size: refused.
    let bad: Answer<Registered> = admin
        .client
        .post(
            "/v1/devices/enrollments",
            &EnrollmentRequest {
                expires_in: None,
                token_hash: Some(vec![1; 31]),
            },
        )
        .await;
    assert_eq!(bad.refused().1.error, "bad_hash");
    let alerts: TokenAlerts = admin
        .client
        .get("/v1/devices/enrollments/alerts")
        .await
        .ok();
    assert!(alerts.alerts.is_empty());

    // The first who comes with it is in; the second is told it served.
    let mut first = Device::enroll(&s, token).await.unwrap();
    assert_eq!(first.team, admin.team);
    let (status, p) = Device::enroll(&s, token).await.err().unwrap();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::CONFLICT, "token_used")
    );
    // The administrators see it, by the hash they registered; a member does not read the alerts.
    let alerts: TokenAlerts = admin
        .client
        .get("/v1/devices/enrollments/alerts")
        .await
        .ok();
    assert_eq!(alerts.alerts.len(), 1);
    assert_eq!(alerts.alerts[0].token_hash, token_hash(token).to_vec());
    assert!(alerts.alerts[0].reused_at >= alerts.alerts[0].used_at);
    first.sign_in().await.unwrap();
    let r: Answer<TokenAlerts> = first.client.get("/v1/devices/enrollments/alerts").await;
    assert_eq!(r.refused().0, StatusCode::FORBIDDEN);
    // A member registers no token.
    assert_eq!(
        register(&first, "another-token-of-a-member-000000001", None)
            .await
            .refused()
            .0,
        StatusCode::FORBIDDEN
    );
    // Another team's administrator does not see this team's alerts.
    let other = s.new_team().await;
    let alerts: TokenAlerts = other
        .client
        .get("/v1/devices/enrollments/alerts")
        .await
        .ok();
    assert!(alerts.alerts.is_empty());

    // Expired: unknown, as before.
    let late = "derived-from-the-secret-after-the-hash-0002";
    register(&admin, late, Some(1)).await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    assert_eq!(
        Device::enroll(&s, late).await.err().unwrap().1.error,
        "bad_token"
    );
    s.finish().await;
}

/// A CBOR body posted, its status only (the routes that answer 204).
async fn post_status(client: &Client, path: &str, body: &impl serde::Serialize) -> StatusCode {
    client
        .request(Method::POST, path)
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(body))
        .send()
        .await
        .unwrap()
        .status()
}

async fn set_role(admin: &Device, device: &Device, role: Role) -> StatusCode {
    post_status(
        &admin.client,
        &format!("/v1/devices/{}/role", device.id_hex()),
        &RoleRequest { role },
    )
    .await
}

async fn the_roles_follow_the_administrator_and_one_stays(b: Backend) {
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let carol = s.add_device(&alice).await;
    // A member does not change a role.
    assert_eq!(
        set_role(&bob, &bob, Role::Admin).await,
        StatusCode::FORBIDDEN
    );
    // Bob, named administrator in the roster, becomes one on the server: he then adds and revokes.
    assert_eq!(
        set_role(&alice, &bob, Role::Admin).await,
        StatusCode::NO_CONTENT
    );
    let devices: Devices = alice.client.get("/v1/devices").await.ok();
    assert!(devices
        .devices
        .iter()
        .any(|d| d.id == bob.id && d.role == Role::Admin));
    assert_eq!(bob.revoke(&carol).await, StatusCode::NO_CONTENT);
    // Back to member, then Alice alone: she cannot make herself a member, the team would have no administrator.
    assert_eq!(
        set_role(&alice, &bob, Role::Member).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        set_role(&alice, &alice, Role::Member).await,
        StatusCode::CONFLICT
    );
    // Another team's device: not found.
    let z = s.new_team().await;
    assert_eq!(
        set_role(&alice, &z, Role::Admin).await,
        StatusCode::NOT_FOUND
    );
    s.finish().await;
}

async fn the_recovery_token_enrols_an_administrator_of_the_team(b: Backend) {
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let team: [u8; 16] = alice.team.clone().try_into().unwrap();
    let token = s.server.new_recovery_token(team).await.unwrap();
    let mut rescuer = Device::enroll(&s, &token).await.unwrap();
    assert_eq!(rescuer.team, alice.team);
    rescuer.sign_in().await.unwrap();
    let devices: Devices = rescuer.client.get("/v1/devices").await.ok();
    assert!(devices
        .devices
        .iter()
        .any(|d| d.id == rescuer.id && d.role == Role::Admin));
    assert_eq!(
        Device::enroll(&s, &token).await.err().unwrap().1.error,
        "token_used"
    );
    s.finish().await;
}

async fn the_pairing_relay_takes_one_try(b: Backend) {
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let other = s.new_team().await;
    let open = s.client();
    let opened: PairOpened = alice
        .client
        .post(
            "/v1/pair",
            &PairOpen {
                message: vec![1; 33],
            },
        )
        .await
        .ok();
    assert_eq!(opened.slot.len(), 5);
    let slot = opened.slot.clone();
    // The new device, not enrolled, reads the first message.
    let m: PairMessage = open
        .get(&format!("/v1/pair/{}", slot.to_lowercase()))
        .await
        .ok();
    assert_eq!(m.message, vec![1; 33]);
    // Nothing tried yet; another device does not read the slot.
    assert_eq!(
        alice
            .client
            .status(Method::GET, &format!("/v1/pair/{slot}/try"), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        other
            .client
            .status(Method::GET, &format!("/v1/pair/{slot}/try"), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    // One try; a second is refused.
    let try_path = format!("/v1/pair/{slot}/try");
    let first = PairTry {
        message: vec![2; 33],
        confirm: vec![3; 32],
    };
    let token = open
        .post::<_, PairTried>(&try_path, &first)
        .await
        .ok()
        .token;
    assert_eq!(token.len(), 32);
    let second = PairTry {
        message: vec![4; 33],
        confirm: vec![5; 32],
    };
    assert_eq!(
        post_status(&open, &try_path, &second).await,
        StatusCode::CONFLICT
    );
    let tried: PairTry = alice.client.get(&format!("/v1/pair/{slot}/try")).await.ok();
    assert_eq!((tried.message, tried.confirm), (vec![2; 33], vec![3; 32]));
    // No answer yet; then the answer, taken once, with the token of the try only.
    let take = |token: &str| PairTake {
        token: token.to_string(),
    };
    let take_path = format!("/v1/pair/{slot}/take");
    assert_eq!(
        post_status(&open, &take_path, &take(&token)).await,
        StatusCode::NO_CONTENT
    );
    let answer = PairAnswer {
        sealed: vec![7; 100],
    };
    assert_eq!(
        post_status(&alice.client, &format!("/v1/pair/{slot}/answer"), &answer).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        post_status(&open, &take_path, &take(&"0".repeat(32))).await,
        StatusCode::FORBIDDEN
    );
    let a: PairAnswer = open.post(&take_path, &take(&token)).await.ok();
    assert_eq!(a.sealed, vec![7; 100]);
    assert_eq!(
        open.status(Method::GET, &format!("/v1/pair/{slot}"), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );

    // A slot closed by its device (a wrong code) is gone; opening one needs a session.
    let opened: PairOpened = alice
        .client
        .post(
            "/v1/pair",
            &PairOpen {
                message: vec![1; 33],
            },
        )
        .await
        .ok();
    assert_eq!(
        alice
            .client
            .status(Method::DELETE, &format!("/v1/pair/{}", opened.slot), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        open.status(Method::GET, &format!("/v1/pair/{}", opened.slot), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let r: Answer<PairOpened> = open
        .post(
            "/v1/pair",
            &PairOpen {
                message: vec![1; 33],
            },
        )
        .await;
    assert_eq!(r.refused().0, StatusCode::UNAUTHORIZED);
    s.finish().await;
}

#[tokio::test]
async fn the_page_of_an_old_invitation_link_reads_nothing_of_it_and_runs_no_script() {
    // SR2-08: the secret of an invitation never passes through a page of the server. A link of before opened in a
    // browser gets a fixed text that says to ask for a new one: no script (the policy forbids any), nothing that
    // reads what follows the `#`, no resource from elsewhere.
    let s = TestServer::start(&Backend::Sqlite).await;
    let id = "00112233445566778899aabbccddeeff";
    let r = s
        .client()
        .http
        .get(format!("{}/join/{id}", s.base))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let csp = r.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .to_string();
    assert!(csp.starts_with("default-src 'none'"), "{csp}");
    assert!(
        csp.contains("script-src 'none'") && !csp.contains("nonce"),
        "{csp}"
    );
    assert_eq!(r.headers()["referrer-policy"], "no-referrer");
    let page = r.text().await.unwrap();
    let lower = page.to_ascii_lowercase();
    for never in [
        "<script",
        "location",
        "hash",
        " on",
        "javascript:",
        "src=",
        "href=",
        "http://",
        "<a ",
        "<form",
    ] {
        assert!(!lower.contains(never), "{never} in the page");
    }
    assert!(page.contains("Demande un nouveau lien"));
    assert!(page.contains("escouade://"));
    // The same page for every id: nothing of the link in it.
    let other = s
        .client()
        .http
        .get(format!("{}/join/ffeeddccbbaa99887766554433221100", s.base))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(other, page);
    assert!(!page.contains(id));
    let r = s
        .client()
        .http
        .get(format!("{}/join/nope", s.base))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    s.finish().await;
}
