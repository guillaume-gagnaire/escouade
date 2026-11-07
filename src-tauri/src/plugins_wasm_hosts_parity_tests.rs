//! The parity of the code hosts in WASM (ESC-345): GitHub, GitLab, Bitbucket, Gitea and Azure DevOps answer every call of
//! `forge/*` and of their sources as their processes did, the `escouade-plugins` of before their port, whose answers and
//! requests `tests/fixtures/plugins/parity/<name>-host.json` holds: on the fixtures of `tests/fixtures/forges/`, nominal,
//! 401, 404, 429, a redirected read and a redirected write. `ESCOUADE_PARITY_RECORD=1` and
//! `ESCOUADE_NATIVE_PLUGINS=<a program of then>` write them again.
//!
//! The account's `api` is the fake's address, for the two: what is compared is what each does of the same words.

use crate::integrations::fake::FakeServer;
use crate::plugins::wasm::{Fetcher, Net};
use crate::plugins_wasm_parity_tests::{
    call_with, hello, module, root, Hosted, Native, Plugin, HEADERS_OF_THE_CLIENT,
};
use serde_json::{json, Value};
use std::collections::HashMap;

/// A step of a transcript.
enum Step {
    Call(&'static str, Value),
    /// What the fake answers from now on.
    Fake(Box<dyn Fn(&FakeServer)>),
}

use Step::{Call, Fake};

fn fixture(kind: &str, file: &str) -> Value {
    let path = root().join("tests/fixtures/forges").join(kind).join(file);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn text_fixture(kind: &str, file: &str) -> String {
    std::fs::read_to_string(root().join("tests/fixtures/forges").join(kind).join(file)).unwrap()
}

/// A code host, on its fixtures.
struct Host {
    /// Its folder of `plugins/` and its name in `escouade-plugins`.
    name: &'static str,
    id: &'static str,
    /// Its code host's id.
    forge: &'static str,
    /// The account's address.
    url: &'static str,
    email: &'static str,
    repo: (&'static str, &'static str),
    number: u64,
    branch: &'static str,
    job: &'static str,
    comment: &'static str,
    /// Where it asks who the token is.
    me: &'static str,
    /// Where it reads the pull request, and what the fake answers there.
    pull: &'static str,
    pull_body: fn() -> Value,
    /// Where a comment is written.
    commented: &'static str,
    routes: fn(&FakeServer),
}

fn github_routes(s: &FakeServer) {
    let fx = |f| fixture("github", f);
    let (sha, repo) = ("6ec1ba54542baca6cc5932582634104c83f36a28", "/repos/cli/cli");
    s.on("GET", "/user", 200, json!({ "login": "ada" }));
    s.on("GET", repo, 200, json!({ "name": "cli", "allow_squash_merge": true, "allow_auto_merge": true, "fork": true, "parent": { "full_name": "cli/upstream" } }));
    s.on("GET", &format!("{repo}/pulls/14768"), 200, fx("pull.json"));
    s.on("POST", "/graphql#Pulls", 200, fx("graphql-pulls.json"));
    s.on(
        "POST",
        "/graphql#PullsOfBranch",
        200,
        fx("graphql-branch.json"),
    );
    s.on("POST", "/graphql#Threads", 200, fx("graphql-threads.json"));
    s.on("POST", "/graphql#OpenPulls", 200, fx("graphql-pulls.json"));
    s.on("POST", "/graphql#RepoParent", 200, json!({ "data": { "repository": { "parent": { "name": "cli", "owner": { "login": "upstream" } } } } }));
    s.on(
        "GET",
        &format!("{repo}/commits/trunk/check-runs"),
        200,
        fx("check-runs.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/commits/trunk/status"),
        200,
        fx("status.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/deployments/7229954119/statuses"),
        200,
        fx("deployment-statuses.json"),
    );
    s.on(
        "POST",
        "/graphql#PullRequestId",
        200,
        json!({ "data": { "repository": { "pullRequest": { "id": "PR_1" } } } }),
    );
    s.on(
        "POST",
        "/graphql#EnableAutoMerge",
        200,
        json!({ "data": { "enablePullRequestAutoMerge": { "pullRequest": { "number": 14768 } } } }),
    );
    s.on(
        "GET",
        &format!("{repo}/commits/{sha}/check-runs"),
        200,
        fx("check-runs.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/commits/{sha}/status"),
        200,
        fx("status.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/pulls/14768/reviews"),
        200,
        fx("reviews.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/issues/14768/comments"),
        200,
        fx("issue-comments.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/deployments"),
        200,
        fx("deployments.json"),
    );
    s.on(
        "PUT",
        &format!("{repo}/pulls/14768/merge"),
        200,
        json!({ "merged": true }),
    );
    s.on("POST", &format!("{repo}/pulls"), 201, fx("pull.json"));
    s.on(
        "POST",
        &format!("{repo}/issues/14768/comments"),
        201,
        json!({ "id": 1 }),
    );
    s.on(
        "POST",
        &format!("{repo}/pulls/14768/reviews"),
        200,
        json!({ "id": 1 }),
    );
    s.on(
        "POST",
        &format!("{repo}/pulls/14768/comments/4335329893/replies"),
        201,
        json!({ "id": 2 }),
    );
    s.on_text(
        "GET",
        &format!("{repo}/actions/jobs/119211823435/logs"),
        200,
        &text_fixture("github", "job-log.txt"),
        &[],
    );
}

fn gitlab_routes(s: &FakeServer) {
    let fx = |f| fixture("gitlab", f);
    let p = "/projects/gitlab-org%2Fapi%2Fclient-go";
    let mr = format!("{p}/merge_requests/3101");
    s.on("GET", "/user", 200, json!({ "username": "ada" }));
    s.on(
        "GET",
        "/personal_access_tokens/self",
        200,
        json!({ "scopes": ["api"] }),
    );
    s.on("GET", p, 200, fx("project.json"));
    s.on("GET", &mr, 200, fx("merge-request.json"));
    s.on("GET", &format!("{mr}/approvals"), 200, fx("approvals.json"));
    s.on("GET", &format!("{mr}/reviewers"), 200, fx("reviewers.json"));
    s.on(
        "GET",
        &format!("{mr}/discussions"),
        200,
        fx("discussions.json"),
    );
    s.on(
        "GET",
        &format!("{mr}/pipelines"),
        200,
        fx("mr-pipelines.json"),
    );
    s.on("GET", "/projects/65275361", 200, fx("fork-project.json"));
    s.on(
        "GET",
        "/projects/65271576/pipelines/3004244757/jobs",
        200,
        fx("jobs.json"),
    );
    s.on(
        "GET",
        &format!("{p}/merge_requests"),
        200,
        fx("branch-mrs.json"),
    );
    s.on(
        "GET",
        &format!("{p}/deployments"),
        200,
        fx("deployments.json"),
    );
    s.on("PUT", &format!("{mr}/merge"), 200, fx("merge-request.json"));
    s.on(
        "POST",
        &format!("{p}/merge_requests"),
        201,
        fx("merge-request.json"),
    );
    s.on("POST", &format!("{mr}/notes"), 201, json!({ "id": 1 }));
    s.on("POST", &format!("{mr}/approve"), 201, json!({}));
    s.on(
        "POST",
        &format!("{mr}/discussions"),
        201,
        json!({ "id": "d1" }),
    );
    s.on(
        "GET",
        &format!("{p}/pipelines"),
        200,
        fx("mr-pipelines.json"),
    );
    s.on(
        "POST",
        &format!("{mr}/discussions/6a9c17/notes"),
        201,
        json!({ "id": 2 }),
    );
    s.on("PUT", &format!("{mr}/discussions/6a9c17"), 200, json!({}));
    s.on_text(
        "GET",
        &format!("{p}/jobs/42/trace"),
        200,
        &text_fixture("gitlab", "job-log.txt"),
        &[],
    );
}

fn bitbucket_routes(s: &FakeServer) {
    let fx = |f| fixture("bitbucket", f);
    let repo = "/repositories/acme/demo";
    let pr = format!("{repo}/pullrequests");
    s.on("GET", "/user", 200, fx("user.json"));
    s.on(
        "GET",
        repo,
        200,
        json!({ "full_name": "acme/demo", "parent": { "full_name": "acme/upstream" } }),
    );
    s.on("GET", &pr, 200, fx("pullrequests-open.json"));
    s.on("GET", &format!("{pr}/42"), 200, fx("pullrequest.json"));
    s.on(
        "GET",
        &format!("{pr}/42/comments"),
        200,
        fx("comments.json"),
    );
    s.on(
        "GET",
        &format!("{pr}/42/statuses"),
        200,
        fx("statuses.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/environments"),
        200,
        fx("environments.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/deployments"),
        200,
        fx("deployments.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/refs/branches/feature"),
        200,
        fx("branch.json"),
    );
    s.on("POST", &pr, 201, fx("pullrequest.json"));
    s.on(
        "POST",
        &format!("{pr}/42/merge"),
        200,
        fx("pullrequest.json"),
    );
    s.on(
        "POST",
        &format!("{pr}/42/comments"),
        201,
        json!({ "id": 1 }),
    );
    s.on("POST", &format!("{pr}/42/approve"), 200, json!({}));
    s.on(
        "POST",
        &format!("{pr}/42/comments/7/resolve"),
        200,
        json!({}),
    );
}

fn gitea_routes(s: &FakeServer) {
    let fx = |f| fixture("gitea", f);
    let repo = "/repos/ada/notes";
    s.on("GET", "/version", 200, json!({ "version": "1.26.0" }));
    s.on("GET", "/user", 200, fx("user.json"));
    s.on("GET", repo, 200, fx("repo.json"));
    s.on("GET", &format!("{repo}/pulls"), 200, fx("pulls-all.json"));
    s.on("GET", &format!("{repo}/pulls/7"), 200, fx("pull.json"));
    s.on(
        "GET",
        &format!("{repo}/pulls/7/reviews"),
        200,
        fx("reviews.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/pulls/7/reviews/12/comments"),
        200,
        fx("review-12-comments.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/issues/7/comments"),
        200,
        fx("issue-comments.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/branches/main"),
        200,
        fx("branch-main.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/branches/dev"),
        200,
        fx("branch-dev.json"),
    );
    s.on("GET", &format!("{repo}/labels"), 200, fx("labels.json"));
    let sha = "3f2a9c1d5e7b84a6f0c2d9e8b7a6543210fedcba";
    s.on(
        "GET",
        &format!("{repo}/commits/{sha}/status"),
        200,
        fx("combined-status.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/commits/{sha}/statuses"),
        200,
        fx("statuses.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/actions/runs/900/jobs"),
        200,
        fx("gitea-run-jobs.json"),
    );
    s.on(
        "POST",
        &format!("{repo}/pulls/7/comments/301/replies"),
        201,
        json!({ "id": 5 }),
    );
    s.on(
        "POST",
        &format!("{repo}/pulls/comments/301/resolve"),
        200,
        Value::Null,
    );
    s.on(
        "GET",
        &format!("{repo}/actions/runs"),
        200,
        fx("gitea-runs.json"),
    );
    s.on("POST", &format!("{repo}/pulls"), 201, fx("pull.json"));
    s.on("POST", &format!("{repo}/pulls/7/merge"), 200, Value::Null);
    s.on(
        "POST",
        &format!("{repo}/issues/7/comments"),
        201,
        json!({ "id": 1 }),
    );
    s.on(
        "POST",
        &format!("{repo}/pulls/7/reviews"),
        200,
        json!({ "id": 1 }),
    );
    s.on_text(
        "GET",
        &format!("{repo}/actions/jobs/9/logs"),
        200,
        &text_fixture("gitea", "job-log.txt"),
        &[],
    );
}

fn azure_routes(s: &FakeServer) {
    let fx = |f| fixture("azure", f);
    let project = "/acme/Demo%20Project/_apis";
    let repo = format!("{project}/git/repositories/demo");
    let pr = format!("{repo}/pullRequests/22");
    s.on(
        "GET",
        "/acme/_apis/connectionData",
        200,
        fx("connection-data.json"),
    );
    s.on(
        "GET",
        &repo,
        200,
        json!({ "id": "r1", "name": "demo", "project": { "id": "p1", "name": "Demo Project" } }),
    );
    s.on("GET", &pr, 200, fx("pull-request.json"));
    s.on(
        "GET",
        &format!("{repo}/pullrequests"),
        200,
        json!({ "value": [fx("pull-request.json")] }),
    );
    s.on("GET", &format!("{pr}/threads"), 200, fx("threads.json"));
    s.on("GET", &format!("{pr}/statuses"), 200, fx("statuses.json"));
    s.on(
        "GET",
        &format!("{project}/policy/evaluations"),
        200,
        fx("evaluations.json"),
    );
    s.on(
        "GET",
        &format!("{project}/policy/configurations"),
        200,
        fx("policy-configurations.json"),
    );
    s.on(
        "GET",
        &format!("{project}/build/builds"),
        200,
        fx("builds.json"),
    );
    s.on(
        "GET",
        &format!("{project}/build/builds/501/timeline"),
        200,
        fx("timeline.json"),
    );
    s.on(
        "GET",
        &format!("{repo}/refs"),
        200,
        json!({ "value": [{ "name": "refs/heads/main", "objectId": "abc123" }] }),
    );
    s.on(
        "GET",
        &format!("{repo}/commits/abc123/statuses"),
        200,
        fx("statuses.json"),
    );
    s.on(
        "POST",
        &format!("{pr}/threads/11/comments"),
        200,
        json!({ "id": 2 }),
    );
    s.on(
        "PATCH",
        &format!("{pr}/threads/11"),
        200,
        json!({ "id": 11 }),
    );
    s.on(
        "PATCH",
        &format!("{project}/build/builds/501"),
        200,
        json!({ "id": 501 }),
    );
    s.on("PATCH", &pr, 200, fx("pull-request.json"));
    s.on(
        "POST",
        &format!("{repo}/pullrequests"),
        201,
        fx("pull-request.json"),
    );
    s.on("POST", &format!("{pr}/threads"), 200, json!({ "id": 1 }));
    s.on_text(
        "GET",
        &format!("{project}/build/builds/501/logs/1"),
        200,
        &text_fixture("azure", "job-log.txt"),
        &[],
    );
}

const HOSTS: [Host; 5] = [
    Host {
        name: "github",
        id: "dev.escouade.github",
        forge: "github",
        url: "https://github.com",
        email: "",
        repo: ("cli", "cli"),
        number: 14768,
        branch: "trunk",
        job: "119211823435",
        comment: "4335329893",
        me: "/user",
        pull: "/repos/cli/cli/pulls/14768",
        pull_body: || fixture("github", "pull.json"),
        commented: "/repos/cli/cli/issues/14768/comments",
        routes: github_routes,
    },
    Host {
        name: "gitlab",
        id: "dev.escouade.gitlab",
        forge: "gitlab",
        url: "https://gitlab.com",
        email: "",
        repo: ("gitlab-org/api", "client-go"),
        number: 3101,
        branch: "feature",
        job: "42",
        comment: "6a9c17:2829102301",
        me: "/user",
        pull: "/projects/gitlab-org%2Fapi%2Fclient-go/merge_requests/3101",
        pull_body: || fixture("gitlab", "merge-request.json"),
        commented: "/projects/gitlab-org%2Fapi%2Fclient-go/merge_requests/3101/notes",
        routes: gitlab_routes,
    },
    Host {
        name: "bitbucket",
        id: "dev.escouade.bitbucket",
        forge: "bitbucket",
        url: "https://bitbucket.org",
        email: "ada@atlas.dev",
        repo: ("acme", "demo"),
        number: 42,
        branch: "feature",
        job: "{1}",
        comment: "7",
        me: "/user",
        pull: "/repositories/acme/demo/pullrequests/42",
        pull_body: || fixture("bitbucket", "pullrequest.json"),
        commented: "/repositories/acme/demo/pullrequests/42/comments",
        routes: bitbucket_routes,
    },
    Host {
        name: "gitea",
        id: "dev.escouade.gitea",
        forge: "gitea",
        url: "https://codeberg.org",
        email: "",
        repo: ("ada", "notes"),
        number: 7,
        branch: "dev",
        job: "9",
        comment: "301",
        me: "/user",
        pull: "/repos/ada/notes/pulls/7",
        pull_body: || fixture("gitea", "pull.json"),
        commented: "/repos/ada/notes/issues/7/comments",
        routes: gitea_routes,
    },
    Host {
        name: "azure-devops",
        id: "dev.escouade.azure-devops",
        forge: "azure",
        url: "https://dev.azure.com/acme",
        email: "",
        repo: ("acme/Demo Project", "demo"),
        number: 22,
        branch: "main",
        job: "501",
        comment: "11:1",
        me: "/acme/_apis/connectionData",
        pull: "/acme/Demo%20Project/_apis/git/repositories/demo/pullRequests/22",
        pull_body: || fixture("azure", "pull-request.json"),
        commented: "/acme/Demo%20Project/_apis/git/repositories/demo/pullRequests/22/threads",
        routes: azure_routes,
    },
];

/// Every call of the contract, then what goes wrong: a refused token, a missing pull request, a limit, a moved one.
fn forge_steps(h: &'static Host, api: &str) -> Vec<Step> {
    let account = json!({ "url": h.url, "email": h.email, "api": api });
    let call = |extra: Value| {
        let mut p = json!({
            "forge": h.forge,
            "account": account,
            "repo": { "namespace": h.repo.0, "name": h.repo.1 },
        });
        p.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        p
    };
    let n = json!({ "number": h.number });
    let me = json!({ "point": "forge", "forge": h.forge, "account": account });
    let draft = json!({
        "title": "Une PR", "body": "Son texte", "head": h.branch, "base": "main", "draft": false,
        "reviewers": ["bob"], "labels": ["bug"], "closes": 9,
    });
    let review = json!({
        "state": "approved", "body": "Bien vu",
        "lines": [{ "path": "src/main.rs", "line": 3, "body": "Ici" }],
    });
    let strategy = json!({ "number": h.number, "strategy": "squash" });
    let moved = format!("{api}/moved");
    let elsewhere = format!("{api}/moved-comment");
    vec![
        Call("account/me", me.clone()),
        Call("forge/pullRequest", call(n.clone())),
        Call(
            "forge/pullRequestsOfBranch",
            call(json!({ "branch": h.branch })),
        ),
        Call("forge/parent", call(json!({}))),
        Call("forge/openPullRequests", call(json!({}))),
        Call("forge/pullRequests", call(json!({ "numbers": [h.number] }))),
        Call("forge/mergeOptions", call(n.clone())),
        Call("forge/checks", call(n.clone())),
        Call("forge/jobLog", call(json!({ "job": h.job }))),
        Call("forge/rerunFailed", call(n.clone())),
        Call("forge/branchChecks", call(json!({ "branch": h.branch }))),
        Call(
            "forge/rerunFailedOfBranch",
            call(json!({ "branch": h.branch })),
        ),
        Call("forge/reviews", call(n.clone())),
        Call("forge/comments", call(n.clone())),
        Call(
            "forge/comment",
            call(json!({ "number": h.number, "body": "Fait.\n\n— Escouade" })),
        ),
        Call(
            "forge/review",
            call(json!({ "number": h.number, "review": review })),
        ),
        Call(
            "forge/reply",
            call(json!({ "number": h.number, "comment": h.comment, "body": "Oui" })),
        ),
        Call(
            "forge/resolve",
            call(json!({ "number": h.number, "comment": h.comment })),
        ),
        Call("forge/deployments", call(json!({ "ref": h.branch }))),
        Call("forge/environments", call(json!({}))),
        Call("forge/createPr", call(json!({ "pr": draft }))),
        Call("forge/merge", call(strategy.clone())),
        Call("forge/autoMerge", call(strategy)),
        Call("forge/nothing", call(json!({}))),
        // A repository moved: a read to its new address, then a write, the token kept on the same host.
        Fake(Box::new({
            let moved = moved.clone();
            move |s| {
                s.on("GET", "/moved", 200, (h.pull_body)());
                s.on_once(
                    "GET",
                    h.pull,
                    301,
                    Value::Null,
                    &[("Location", moved.as_str())],
                );
            }
        })),
        Call("forge/pullRequest", call(n.clone())),
        Fake(Box::new({
            let elsewhere = elsewhere.clone();
            move |s| {
                s.on("POST", "/moved-comment", 201, json!({ "id": 3 }));
                s.on_once(
                    "POST",
                    h.commented,
                    307,
                    Value::Null,
                    &[("Location", elsewhere.as_str())],
                );
            }
        })),
        Call(
            "forge/comment",
            call(json!({ "number": h.number, "body": "Déplacé" })),
        ),
        Fake(Box::new(|s| {
            s.on("GET", h.pull, 404, json!({ "message": "Not Found" }));
        })),
        Call("forge/pullRequest", call(n)),
        // A refused token before the limit: Bitbucket takes its hour as spent after that one.
        Fake(Box::new(|s| {
            s.on("GET", h.me, 401, json!({ "message": "Bad credentials" }));
        })),
        Call("account/me", me.clone()),
        Fake(Box::new(|s| {
            s.on_with(
                "GET",
                h.me,
                429,
                json!({ "message": "Slow down" }),
                &[("retry-after", "1")],
            );
        })),
        Call("account/me", me),
    ]
}

/// A source of the Kanban that a code host's plugin serves.
struct Source {
    name: &'static str,
    id: &'static str,
    source: &'static str,
    /// Its `site`.
    site: &'static str,
    container: &'static str,
    reference: (&'static str, &'static str),
    /// What its fake prefixes its API with.
    routes: fn(&FakeServer),
}

fn github_issue(n: u64, state: &str) -> Value {
    json!({
        "number": n, "title": format!("Issue {n}"), "body": "- [ ] Un critère", "state": state,
        "html_url": format!("https://github.com/acme/api/issues/{n}"),
        "user": { "login": "ada" }, "assignee": null, "labels": [{ "name": "bug" }],
    })
}

fn github_source_routes(s: &FakeServer) {
    let repo = "/repos/acme/api";
    s.on("GET", "/user", 200, json!({ "login": "ada" }));
    s.on(
        "GET",
        "/user/repos",
        200,
        json!([{ "full_name": "acme/api" }]),
    );
    s.on(
        "GET",
        &format!("{repo}/labels"),
        200,
        json!([{ "name": "bug", "color": "ff0000" }, { "name": "todo" }]),
    );
    s.on_with(
        "GET",
        &format!("{repo}/issues"),
        200,
        json!([github_issue(1, "open"), github_issue(2, "open")]),
        &[("Link", "<https://api.github.com/x?page=2>; rel=\"next\"")],
    );
    s.on(
        "GET",
        "/search/issues",
        200,
        json!({ "total_count": 1, "items": [github_issue(3, "open")] }),
    );
    s.on(
        "GET",
        &format!("{repo}/issues/42"),
        200,
        github_issue(42, "open"),
    );
    s.on(
        "PATCH",
        &format!("{repo}/issues/42"),
        200,
        github_issue(42, "closed"),
    );
    s.on("POST", &format!("{repo}/issues/42/labels"), 200, json!([]));
    s.on(
        "DELETE",
        &format!("{repo}/issues/42/labels/todo"),
        200,
        json!([]),
    );
    s.on(
        "POST",
        &format!("{repo}/issues/42/comments"),
        401,
        json!({ "message": "Bad credentials" }),
    );
}

fn gitlab_issue(iid: u64) -> Value {
    json!({
        "iid": iid, "title": format!("Issue {iid}"), "description": "- [ ] Un critère", "state": "opened",
        "web_url": format!("https://gitlab.com/acme/api/-/issues/{iid}"),
        "author": { "username": "ada" }, "assignee": null, "labels": ["bug"],
    })
}

fn gitlab_source_routes(s: &FakeServer) {
    let p = "/projects/acme%2Fapi";
    s.on("GET", "/user", 200, json!({ "username": "ada" }));
    s.on(
        "GET",
        "/projects",
        200,
        json!([{ "path_with_namespace": "acme/api" }]),
    );
    s.on(
        "GET",
        &format!("{p}/labels"),
        200,
        json!([{ "name": "bug" }]),
    );
    s.on(
        "GET",
        &format!("{p}/boards"),
        200,
        json!([{ "lists": [{ "label": { "name": "bug" } }] }]),
    );
    s.on(
        "GET",
        &format!("{p}/issues"),
        200,
        json!([gitlab_issue(1), gitlab_issue(2)]),
    );
    s.on("GET", &format!("{p}/issues/42"), 200, gitlab_issue(42));
    s.on("PUT", &format!("{p}/issues/42"), 200, gitlab_issue(42));
    s.on(
        "POST",
        &format!("{p}/issues/42/notes"),
        429,
        json!({ "message": "Retry later" }),
    );
}

fn azure_source_routes(s: &FakeServer) {
    let org = "/acme/_apis";
    let p = "/acme/Web%20Shop/_apis";
    let state = |name: &str, category: &str| json!({ "name": name, "color": "007acc", "category": category });
    s.on(
        "GET",
        &format!("{org}/connectionData"),
        200,
        fixture("azure", "connection-data.json"),
    );
    s.on(
        "GET",
        &format!("{org}/projects"),
        200,
        json!({ "value": [{ "id": "p1", "name": "Web Shop" }] }),
    );
    s.on(
        "GET",
        &format!("{p}/wit/workitemtypes"),
        200,
        json!({ "value": [{ "name": "Task", "states": [state("New", "Proposed"), state("Active", "InProgress"), state("Closed", "Completed")] }] }),
    );
    s.on(
        "POST",
        &format!("{p}/wit/wiql"),
        200,
        json!({ "workItems": [{ "id": 5 }] }),
    );
    s.on(
        "GET",
        &format!("{p}/wit/workitems"),
        200,
        json!({ "value": [{ "id": 5, "fields": {
            "System.Title": "Une tâche", "System.State": "New", "System.WorkItemType": "Task",
            "System.TeamProject": "Web Shop", "System.Description": "<div>Un texte</div>",
        } }] }),
    );
    s.on(
        "GET",
        &format!("{p}/wit/workitems/5"),
        200,
        json!({ "id": 5, "fields": { "System.State": "New", "System.WorkItemType": "Task" } }),
    );
    s.on(
        "PATCH",
        &format!("{p}/wit/workitems/5"),
        200,
        json!({ "id": 5 }),
    );
    s.on(
        "POST",
        &format!("{p}/wit/workitems/5/comments"),
        403,
        json!({ "message": "TF: denied" }),
    );
}

const SOURCES: [Source; 3] = [
    Source {
        name: "github",
        id: "dev.escouade.github",
        source: "github",
        site: "https://github.com",
        container: "acme/api",
        reference: ("42", "acme/api#42"),
        routes: github_source_routes,
    },
    Source {
        name: "gitlab",
        id: "dev.escouade.gitlab",
        source: "gitlab",
        site: "https://gitlab.com",
        container: "gitlab.com/acme/api",
        reference: ("42", "gitlab.com/acme/api#42"),
        routes: gitlab_source_routes,
    },
    Source {
        name: "azure-devops",
        id: "dev.escouade.azure-devops",
        source: "azure",
        site: "https://dev.azure.com/acme",
        container: "acme/Web Shop",
        reference: ("5", "acme/Web Shop#5"),
        routes: azure_source_routes,
    },
];

fn source_steps(s: &'static Source, api: &str) -> Vec<Step> {
    let account =
        json!({ "api": api, "site": s.site, "email": "", "label": "@ada", "user": "ada" });
    let params = |extra: Value| {
        let mut p = json!({ "point": "ticketSource", "source": s.source, "account": account });
        p.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        p
    };
    let reference = json!({
        "service": s.source, "id": s.reference.0, "key": s.reference.1, "url": "", "container": s.container,
    });
    let closed = json!({ "id": "closed", "name": "Closed" });
    let mapped = json!([{ "id": "label:todo", "name": "todo" }, closed]);
    let query = |text: &str, label: Value| json!({ "text": text, "filters": [], "label": label, "page": null });
    vec![
        Call("account/me", params(json!({}))),
        Call("tickets/containers", params(json!({}))),
        Call(
            "tickets/states",
            params(json!({ "container": s.container })),
        ),
        Call(
            "tickets/issues",
            params(json!({ "container": s.container, "query": query("", Value::Null) })),
        ),
        Call(
            "tickets/issues",
            params(json!({ "container": s.container, "query": query("token", json!("bug")) })),
        ),
        Call(
            "tickets/setState",
            params(json!({ "ref": reference, "state": closed, "mapped": mapped })),
        ),
        Call(
            "tickets/setState",
            params(
                json!({ "ref": reference, "state": { "id": "label:bug", "name": "bug" }, "mapped": mapped }),
            ),
        ),
        Call(
            "tickets/comment",
            params(json!({ "ref": reference, "text": "Fait, voir la PR." })),
        ),
        Call("tickets/nothing", params(json!({}))),
    ]
}

/// A budget says when it ends, from the clock of the run; and Bitbucket's what is left by its own count, where a redirection
/// the module follows is the call it is, and the process's was not counted.
fn without_dates(v: Value) -> Value {
    match v {
        Value::Object(o) => o
            .into_iter()
            .map(|(k, v)| {
                let kept = if k == "reset" || k == "remaining" {
                    Value::Null
                } else {
                    without_dates(v)
                };
                (k, kept)
            })
            .collect(),
        Value::Array(a) => a.into_iter().map(without_dates).collect(),
        other => other,
    }
}

/// What a plugin answers each step and what it asked its service meanwhile, in the order of their addresses (Azure asks
/// several at once).
fn transcript(plugin: &mut dyn Plugin, server: &FakeServer, steps: Vec<Step>) -> Value {
    let secret = json!("tok_1");
    let mut greeting = hello();
    greeting["lang"] = json!("fr");
    assert!(call_with(plugin, 1, "initialize", greeting, &secret)["result"].is_object());
    let (mut played, mut seen) = (Vec::new(), 0);
    for (i, step) in steps.into_iter().enumerate() {
        let (method, p) = match step {
            Call(method, p) => (method, p),
            Fake(change) => {
                change(server);
                continue;
            }
        };
        let v = call_with(plugin, 2 + i as u64, method, p, &secret);
        let requests = server.requests();
        let mut asked: Vec<Value> = requests[seen..]
            .iter()
            .map(|r| {
                let mut headers: Vec<(&String, &String)> = r
                    .headers
                    .iter()
                    .filter(|(k, _)| !HEADERS_OF_THE_CLIENT.contains(&k.as_str()) && *k != "referer")
                    .collect();
                headers.sort();
                json!({ "method": r.method, "target": r.target, "headers": headers, "body": r.body })
            })
            .collect();
        asked.sort_by_key(Value::to_string);
        seen = requests.len();
        played.push(
            json!({ "method": method, "result": v["result"], "error": v["error"], "asked": asked }),
        );
    }
    let bare = server.url.trim_start_matches("http://");
    let text = serde_json::to_string(&played)
        .unwrap()
        .replace(&server.url, "{base}")
        .replace(bare, "{host}");
    without_dates(serde_json::from_str(&text).unwrap())
}

/// What the module of `name` answers, against what its process answered (`file`).
async fn parity(
    name: &str,
    id: &str,
    file: &str,
    routes: fn(&FakeServer),
    steps: impl Fn(&str) -> Vec<Step>,
) {
    let Some(module) = module(name) else {
        return;
    };
    let manifest = root()
        .join("plugins")
        .join(name)
        .join("escouade-plugin.json");
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(manifest).unwrap()).unwrap();
    let network: Vec<String> =
        serde_json::from_value(manifest["permissions"]["network"].clone()).unwrap();

    let server = FakeServer::start().await;
    routes(&server);
    let net = Net {
        routes: HashMap::from([("fake.test".to_string(), server.url.clone())]),
        dns: None,
        ..Net::default()
    };
    let mut hosted = Hosted::start(module, Fetcher::new(id, &network, &net));
    let plan = steps(&server.url);
    let played = tokio::task::block_in_place(|| transcript(&mut hosted, &server, plan));

    let recording = root().join("tests/fixtures/plugins/parity").join(file);
    if std::env::var_os("ESCOUADE_PARITY_RECORD").is_some() {
        let server = FakeServer::start().await;
        routes(&server);
        let mut native = Native::start(name);
        let plan = steps(&server.url);
        let was = tokio::task::block_in_place(|| transcript(&mut native, &server, plan));
        std::fs::write(&recording, serde_json::to_string_pretty(&was).unwrap()).unwrap();
    }
    let was: Value =
        serde_json::from_str(&std::fs::read_to_string(&recording).unwrap()).expect("a recording");
    for (a, b) in played
        .as_array()
        .unwrap()
        .iter()
        .zip(was.as_array().unwrap())
    {
        assert_eq!(a, b, "{name}: {}", a["method"]);
    }
    assert_eq!(played, was, "{name}");
}

async fn host(i: usize) {
    let h: &'static Host = &HOSTS[i];
    parity(
        h.name,
        h.id,
        &format!("{}-host.json", h.name),
        h.routes,
        |api| forge_steps(h, api),
    )
    .await;
}

async fn source(i: usize) {
    let s: &'static Source = &SOURCES[i];
    parity(
        s.name,
        s.id,
        &format!("{}-source.json", s.name),
        s.routes,
        |api| source_steps(s, api),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_answers_as_its_native_plugin() {
    host(0).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gitlab_answers_as_its_native_plugin() {
    host(1).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn bitbucket_answers_as_its_native_plugin() {
    host(2).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gitea_answers_as_its_native_plugin() {
    host(3).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn azure_devops_answers_as_its_native_plugin() {
    host(4).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_issues_answer_as_their_native_plugin() {
    source(0).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gitlab_issues_answer_as_their_native_plugin() {
    source(1).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn azure_boards_answer_as_their_native_plugin() {
    source(2).await;
}
