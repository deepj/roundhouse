//! A route helper takes its positional arguments in the order its path
//! carries the dynamic segments, as Rails does. Under a dynamic scope
//! (`scope ":slug"`) the scope's segment leads, for every route in it:
//! `preview_article_path("acme", 1)` is `/acme/articles/1/preview`.
//!
//! An explicit `member`/`collection` route put the nesting's segments
//! first and the scope's last (`preview_article_path(id, slug)`), so the
//! call Rails answers with `/acme/articles/1/preview` built
//! `/1/articles/acme/preview`; and a `root` inside the scope had no
//! params at all, so `root_path` built the literal `/:slug` where Rails
//! recalls the request's own (`/acme`). Answers are actionpack 8.1.4's
//! (`rails_answers_the_table`).

#[path = "support/emit_and_run.rs"]
#[allow(dead_code)]
mod emit_and_run;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use roundhouse::ingest::ingest_app_from_tree;
use roundhouse::lower::lower_routes_to_library_functions;
use roundhouse::ty::Ty;

const SCHEMA: &str = "ActiveRecord::Schema[8.1].define(version: 1) do\nend\n";

const APPLICATION: &str = "class ApplicationController < ActionController::Base\nend\n";

const ARTICLES: &str = r#"class ArticlesController < ApplicationController
  def index
    render plain: "index"
  end

  def show
    render plain: "show"
  end

  def preview
    render plain: root_path
  end

  def purge
    render plain: "purge"
  end
end
"#;

const COMMENTS: &str = r#"class CommentsController < ApplicationController
  def show
    render plain: "comment"
  end

  def quote
    render plain: "quote"
  end

  def recent
    render plain: "recent"
  end
end
"#;

const DRAFTS: &str = r#"class Desk::DraftsController < ApplicationController
  def peek
    render plain: "peek"
  end
end
"#;

const NOTES: &str = r#"class NotesController < ApplicationController
  def peek
    render plain: "peek"
  end
end
"#;

const REPLIES: &str = r#"class RepliesController < ApplicationController
  def quote
    render plain: "quote"
  end
end
"#;

const ROUTES: &str = r#"Rails.application.routes.draw do
  scope ":slug" do
    root "articles#index"
    resources :articles, only: [:show] do
      member do
        get :preview
      end
      collection do
        delete :purge
      end
      resources :comments, only: [:show] do
        member do
          get :quote
        end
        collection do
          get :recent
        end
      end
    end
    namespace :desk do
      resources :drafts, only: [] do
        get :peek, on: :member
      end
    end
  end
  resources :notes, only: [] do
    get :peek, on: :member
    resources :replies, only: [] do
      get :quote, on: :member
    end
  end
end
"#;

/// `(helper, its positional params, a call, the URL it builds)`.
const HELPERS: &[(&str, &[&str], &str, &str)] = &[
    ("root_path", &["slug"], r#"root_path("acme")"#, "/acme"),
    ("article_path", &["slug", "id"], r#"article_path("acme", 1)"#, "/acme/articles/1"),
    ("preview_article_path", &["slug", "id"], r#"preview_article_path("acme", 1)"#, "/acme/articles/1/preview"),
    ("purge_articles_path", &["slug"], r#"purge_articles_path("acme")"#, "/acme/articles/purge"),
    (
        "article_comment_path",
        &["slug", "article_id", "id"],
        r#"article_comment_path("acme", 1, 2)"#,
        "/acme/articles/1/comments/2",
    ),
    (
        "quote_article_comment_path",
        &["slug", "article_id", "id"],
        r#"quote_article_comment_path("acme", 1, 2)"#,
        "/acme/articles/1/comments/2/quote",
    ),
    (
        "recent_article_comments_path",
        &["slug", "article_id"],
        r#"recent_article_comments_path("acme", 1)"#,
        "/acme/articles/1/comments/recent",
    ),
    ("peek_desk_draft_path", &["slug", "id"], r#"peek_desk_draft_path("acme", 3)"#, "/acme/desk/drafts/3/peek"),
    ("peek_note_path", &["id"], "peek_note_path(4)", "/notes/4/peek"),
    ("quote_note_reply_path", &["note_id", "id"], "quote_note_reply_path(4, 5)", "/notes/4/replies/5/quote"),
];

/// A zero-argument `root_path` in a request under the scope recalls the
/// request's `:slug`.
const RECALL: (&str, &str) = ("/acme/articles/1/preview", "/acme");

fn tree() -> HashMap<PathBuf, Vec<u8>> {
    [
        ("db/schema.rb", SCHEMA),
        ("app/controllers/application_controller.rb", APPLICATION),
        ("app/controllers/articles_controller.rb", ARTICLES),
        ("app/controllers/comments_controller.rb", COMMENTS),
        ("app/controllers/desk/drafts_controller.rb", DRAFTS),
        ("app/controllers/notes_controller.rb", NOTES),
        ("app/controllers/replies_controller.rb", REPLIES),
        ("config/routes.rb", ROUTES),
    ]
    .into_iter()
    .map(|(p, s)| (PathBuf::from(p), s.as_bytes().to_vec()))
    .collect()
}

/// Every call in `HELPERS`, then the `RECALL` request (`request` is the
/// Ruby expression answering it as `[status, body]`), each mismatch
/// collected so a failure names them all.
fn assertions(call_prefix: &str, request: &str) -> String {
    let (path, want) = RECALL;
    let mut out = String::from("failures = []\n");
    for (_, _, call, want) in HELPERS {
        out.push_str(&format!(
            "got = begin\n  {call_prefix}{call}\nrescue => e\n  \"#{{e.class}}: #{{e.message}}\"\nend\n\
             failures << {call:?} + \" => #{{got}}\" unless got == {want:?}\n"
        ));
    }
    out.push_str(&format!(
        "status, text = {request}\n\
         failures << \"GET {path} => #{{status}} #{{text}}\" unless status == 200 && text == {want:?}\n\
         raise failures.join(\"\\n\") unless failures.empty?\n"
    ));
    out
}

#[test]
fn helper_params_follow_the_paths_segment_order() {
    let app = ingest_app_from_tree(tree()).expect("ingest tree");
    let helpers = lower_routes_to_library_functions(&app);
    let mut wrong = Vec::new();
    for (name, want, _, _) in HELPERS {
        let f = helpers
            .iter()
            .find(|f| f.name.as_str() == *name)
            .unwrap_or_else(|| panic!("helper {name} not generated"));
        let Ty::Fn { params, .. } = f.signature.clone().expect("signature") else {
            panic!("{name}: not a Ty::Fn")
        };
        let got: Vec<String> = params.iter().map(|p| p.name.as_str().to_string()).collect();
        if got != *want {
            wrong.push(format!("{name}{got:?}"));
        }
    }
    assert!(wrong.is_empty(), "params out of path order: {wrong:?}");
}

#[test]
fn the_ruby_target_builds_the_urls_rails_does() {
    let request = format!(
        "Main.run_rack(\"REQUEST_METHOD\" => \"GET\", \"PATH_INFO\" => {:?}, \"QUERY_STRING\" => \"\", \"rack.input\" => StringIO.new(\"\")).then {{ |r| [r[0], r[2].join] }}",
        RECALL.0
    );
    emit_and_run::empty_app()
        .write("db/schema.rb", SCHEMA)
        .write("app/controllers/application_controller.rb", APPLICATION)
        .write("app/controllers/articles_controller.rb", ARTICLES)
        .write("app/controllers/comments_controller.rb", COMMENTS)
        .write("app/controllers/desk/drafts_controller.rb", DRAFTS)
        .write("app/controllers/notes_controller.rb", NOTES)
        .write("app/controllers/replies_controller.rb", REPLIES)
        .write("config/routes.rb", ROUTES)
        .run_ruby(&format!(
            r#"require "stringio"
require_relative "app/route_helpers"
{}puts "PASS"
"#,
            assertions("RouteHelpers.", &request)
        ))
        .assert_passes();
}

/// The table itself, against actionpack 8.1.4's router.
#[test]
fn rails_answers_the_table() {
    let probe = r#"gem "actionpack", "8.1.4"; require "action_controller""#;
    if !Command::new("ruby").args(["-e", probe]).status().is_ok_and(|s| s.success()) {
        eprintln!("skipping: ruby with actionpack 8.1.4 not available");
        return;
    }
    let script = format!(
        r##"gem "actionpack", "8.1.4"
require "action_controller"
require "rack/mock"
ROUTES = ActionDispatch::Routing::RouteSet.new
ROUTES.draw do
{draw}
end
class ApplicationController < ActionController::Base
  include ROUTES.url_helpers
end
module Desk; end
{ARTICLES}
{COMMENTS}
{DRAFTS}
{NOTES}
{REPLIES}
def req(path)
  status, _headers, body = ROUTES.call(Rack::MockRequest.env_for(path))
  text = +""
  body.each {{ |part| text << part }}
  [status, text]
end
h = Class.new {{ include ROUTES.url_helpers }}.new
{asserts}puts "Rails agrees"
"##,
        draw = ROUTES.lines().filter(|l| l.starts_with("  ")).collect::<Vec<_>>().join("\n"),
        asserts = assertions("h.", &format!("req({:?})", RECALL.0)),
    );
    let output = Command::new("ruby").arg("-e").arg(&script).output().expect("ruby is on PATH");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("Rails agrees"),
        "actionpack 8.1.4 disagrees with the table\n=== stdout ===\n{stdout}\n=== stderr ===\n{stderr}"
    );
}
