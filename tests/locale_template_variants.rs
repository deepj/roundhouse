//! A locale-specific template (`welcome.de.html.erb`) is what Rails
//! renders for `welcome` while `I18n.locale` is `de`: its lookup tries
//! the locale, its `I18n.fallbacks` when the app sets them, then the
//! default locale, and a template without a locale after those, a file
//! that names its format before one that does not. Actions, partials,
//! collection partials, layouts and mailer templates all look up so.
//! Roundhouse named such a file `welcome.de`, which nothing rendering
//! `welcome` found: an action with only locale templates answered 204, a
//! partial raised NoMethodError, a localized layout was dropped and a
//! localized mailer template refused the emit.
//!
//! Every answer in the table is actionpack 8.1.4's for the same app
//! (`rails_answers_the_table`).

#[path = "support/scratch.rs"]
mod scratch;
#[path = "support/emit_and_run.rs"]
#[allow(dead_code)]
mod emit_and_run;

use std::path::Path;
use std::process::Command;

use roundhouse::project::BuildTarget;

const I18N: &str = "    config.i18n.available_locales = [:en, :de, :\"de-AT\"]\n    config.i18n.default_locale = :en\n";
const FALLBACKS: &str = "    config.i18n.fallbacks = [:de]\n";

const FILES: &[(&str, &str)] = &[
    ("db/schema.rb", "ActiveRecord::Schema[8.1].define(version: 1) do\n  create_table \"articles\", force: :cascade do |t|\n    t.string \"title\"\n  end\nend\n"),
    ("app/models/application_record.rb", "class ApplicationRecord < ActiveRecord::Base\n  self.abstract_class = true\nend\n"),
    ("app/models/article.rb", "class Article < ApplicationRecord\nend\n"),
    ("config/locales/en.yml", "en:\n  blog: \"Blog\"\n"),
    ("config/locales/de.yml", "de:\n  blog: \"Blog\"\n"),
    (
        "app/controllers/application_controller.rb",
        "class ApplicationController < ActionController::Base\n  around_action :switch_locale\n\n  def switch_locale(&action)\n    I18n.with_locale(params[:locale] || I18n.default_locale, &action)\n  end\nend\n",
    ),
    (
        "app/controllers/pages_controller.rb",
        "class PagesController < ApplicationController\n  def welcome\n    @title = \"readers\"\n  end\n\n  def about\n  end\n\n  def about_de\n  end\n\n  def archive\n    @articles = Article.order(:id)\n  end\n\n  def digest\n    render plain: NewsletterMailer.weekly(\"Ann\").body.to_s.strip\n  end\n\n  def faq\n  end\n\n  def terms\n  end\n\n  def legal\n  end\nend\n",
    ),
    (
        "app/mailers/newsletter_mailer.rb",
        "class NewsletterMailer < ActionMailer::Base\n  def weekly(name)\n    @name = name\n    mail(to: \"reader@example.com\", subject: \"Weekly\")\n  end\nend\n",
    ),
    ("app/views/layouts/application.en.html.erb", "[en]<%= yield %>\n"),
    ("app/views/layouts/application.de.html.erb", "[de]<%= yield %>\n"),
    // Only locale templates.
    ("app/views/pages/welcome.en.html.erb", "<h1>Welcome <%= @title %></h1>\n"),
    ("app/views/pages/welcome.de.html.erb", "<h1>Willkommen <%= @title %></h1>\n"),
    // A template without a locale beside one with.
    ("app/views/pages/about.html.erb", "<p>About us</p>\n"),
    ("app/views/pages/about.de.html.erb", "<p>Ueber uns</p>\n"),
    // Its own template, not `about`'s German one.
    ("app/views/pages/about_de.html.erb", "<p>About, the page</p>\n"),
    // A partial and a collection partial with only locale templates.
    ("app/views/pages/archive.html.erb", "<div><%= render \"notice\" %></div>\n<ul><%= render partial: \"article\", collection: @articles %></ul>\n"),
    ("app/views/pages/_notice.en.html.erb", "<span>Archive</span>\n"),
    ("app/views/pages/_notice.de.html.erb", "<span>Archiv</span>\n"),
    ("app/views/pages/_article.en.html.erb", "<li>Article <%= article.title %></li>\n"),
    ("app/views/pages/_article.de.html.erb", "<li>Artikel <%= article.title %></li>\n"),
    ("app/views/newsletter_mailer/weekly.en.html.erb", "Weekly news for <%= @name %>\n"),
    ("app/views/newsletter_mailer/weekly.de.html.erb", "Wochennews fuer <%= @name %>\n"),
    // The default locale's template before the one without a locale.
    ("app/views/pages/faq.html.erb", "<p>FAQ</p>\n"),
    ("app/views/pages/faq.en.html.erb", "<p>FAQ in English</p>\n"),
    // A file naming its format before one naming only its locale.
    ("app/views/pages/terms.html.erb", "<p>Terms</p>\n"),
    ("app/views/pages/terms.de.erb", "<p>AGB</p>\n"),
    // `js` is a format, not a locale.
    ("app/views/pages/legal.html.erb", "<p>Legal</p>\n"),
    ("app/views/pages/legal.js.erb", "console.log(\"legal\");\n"),
];

/// A path and what it renders in `en` (and with no `locale` param), in
/// `de`, in `de-AT` (whose parent `de` comes before the default locale),
/// and in `en` again when `config.i18n.fallbacks = [:de]` puts `de` after it.
const TABLE: &[(&str, &str, &str, &str, &str)] = &[
    ("welcome", "[en]<h1>Welcome readers</h1>", "[de]<h1>Willkommen readers</h1>", "[de]<h1>Willkommen readers</h1>", "[en]<h1>Welcome readers</h1>"),
    ("about", "[en]<p>About us</p>", "[de]<p>Ueber uns</p>", "[de]<p>Ueber uns</p>", "[en]<p>Ueber uns</p>"),
    ("about_de", "[en]<p>About, the page</p>", "[de]<p>About, the page</p>", "[de]<p>About, the page</p>", "[en]<p>About, the page</p>"),
    (
        "archive",
        "[en]<div><span>Archive</span></div><ul><li>Article one</li><li>Article two</li></ul>",
        "[de]<div><span>Archiv</span></div><ul><li>Artikel one</li><li>Artikel two</li></ul>",
        "[de]<div><span>Archiv</span></div><ul><li>Artikel one</li><li>Artikel two</li></ul>",
        "[en]<div><span>Archive</span></div><ul><li>Article one</li><li>Article two</li></ul>",
    ),
    ("digest", "Weekly news for Ann", "Wochennews fuer Ann", "Wochennews fuer Ann", "Weekly news for Ann"),
    ("faq", "[en]<p>FAQ in English</p>", "[de]<p>FAQ in English</p>", "[de]<p>FAQ in English</p>", "[en]<p>FAQ in English</p>"),
    ("terms", "[en]<p>Terms</p>", "[de]<p>Terms</p>", "[de]<p>Terms</p>", "[en]<p>Terms</p>"),
    ("legal", "[en]<p>Legal</p>", "[de]<p>Legal</p>", "[de]<p>Legal</p>", "[en]<p>Legal</p>"),
];

fn files(fallbacks: bool) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = FILES.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect();
    let config = format!("{I18N}{}", if fallbacks { FALLBACKS } else { "" });
    out.push((
        "config/application.rb".into(),
        format!("require_relative \"boot\"\nrequire \"rails/all\"\n\nmodule Blog\n  class Application < Rails::Application\n    config.load_defaults 8.1\n{config}  end\nend\n"),
    ));
    let mut routes = String::from("Rails.application.routes.draw do\n");
    for (path, ..) in TABLE {
        routes.push_str(&format!("  get \"{path}\", to: \"pages#{path}\"\n"));
    }
    routes.push_str("end\n");
    out.push(("config/routes.rb".into(), routes));
    out
}

/// The Ruby that requests every row in every locale through `call` (an
/// expression of `env` answering Rack's triple) and raises naming each
/// mismatch.
fn assertions(call: &str, fallbacks: bool) -> String {
    let mut out = format!(
        "def req(env)\n  status, _headers, body = {call}\n  text = +\"\"\n  body.each {{ |part| text << part }}\n  status == 200 ? text.gsub(/\\s*\\n\\s*/, \"\") : \"!#{{status}}\"\nrescue Exception => e\n  \"!#{{e.class}}: #{{e.message}}\"\nend\nArticle.create!(title: \"one\")\nArticle.create!(title: \"two\")\nfailures = []\n"
    );
    for (path, en, de, de_at, en_falling_back) in TABLE {
        let en = if fallbacks { en_falling_back } else { en };
        for (query, want) in [("", en), ("locale=en", en), ("locale=de", de), ("locale=de-AT", de_at)] {
            out.push_str(&format!(
                "got = req([{path:?}, {query:?}])\nfailures << \"GET /{path}?{query} => #{{got}}, want {want}\" unless got == {want:?}\n"
            ));
        }
    }
    out.push_str("raise failures.join(\"\\n\") unless failures.empty?\n");
    out
}

const RACK_ENV: &str = "{ \"REQUEST_METHOD\" => \"GET\", \"PATH_INFO\" => \"/\" + env[0], \"QUERY_STRING\" => env[1], \"HTTP_ACCEPT\" => \"text/html\", \
     \"rack.input\" => StringIO.new(\"\"), \"SERVER_NAME\" => \"example.org\", \"SERVER_PORT\" => \"80\", \"rack.url_scheme\" => \"http\" }";

fn app(fallbacks: bool) -> emit_and_run::Overlay {
    files(fallbacks).into_iter().fold(emit_and_run::empty_app(), |app, (path, body)| app.write(&path, &body))
}

fn ruby_target_run(fallbacks: bool) {
    app(fallbacks)
        .run_ruby(&format!(
            "require \"stringio\"\n{}puts \"PASS\"\n",
            assertions(&format!("Main.run_rack({RACK_ENV})"), fallbacks)
        ))
        .assert_passes();
}

#[test]
fn each_locale_renders_its_template() {
    ruby_target_run(false);
}

#[test]
fn configured_fallbacks_come_after_the_locale() {
    ruby_target_run(true);
}

/// The variants are one template to every target: one view per name, no
/// locale left in a name.
#[test]
fn the_variants_are_one_view() {
    let mut tree: std::collections::HashMap<std::path::PathBuf, Vec<u8>> =
        files(false).into_iter().map(|(p, s)| (p.into(), s.into_bytes())).collect();
    tree.insert("app/views/pages/x.html.erb".into(), b"x\n".to_vec());
    let app = roundhouse::ingest::ingest_app_from_tree(tree).expect("ingest");
    let mut views: Vec<String> = app.views.iter().map(|v| format!("{} {}", v.name.as_str(), v.format.as_str())).collect();
    views.sort();
    assert_eq!(
        views,
        [
            "layouts/application html",
            "newsletter_mailer/weekly html",
            "pages/_article html",
            "pages/_notice html",
            "pages/about html",
            "pages/about_de html",
            "pages/archive html",
            "pages/faq html",
            "pages/legal html",
            "pages/legal js",
            "pages/terms html",
            "pages/welcome html",
            "pages/x html",
        ]
    );
}

/// What the choice cannot be made for is an error, not a guess.
#[test]
fn what_is_not_modeled_is_an_error() {
    let errors = |extra: &[(&str, &str)]| -> Vec<String> {
        let app = extra.iter().fold(app(false), |app, (path, body)| app.write(path, body));
        app.check_errors().into_iter().filter(|e| e.contains("locale_template")).collect()
    };
    assert_eq!(errors(&[]), Vec::<String>::new());
    let cases: &[(&[(&str, &str)], &str)] = &[
        (&[("app/views/pages/imprint.de.html.erb", "<p>Impressum</p>\n")], "for `en` Rails finds none of them"),
        (&[("app/views/pages/feed.de.json.jbuilder", "json.title \"Feed\"\n")], "a Jbuilder template's locale variants"),
        (
            &[
                ("app/views/pages/_card.en.html.erb", "<%# locals: (title:) -%>\n<b><%= title %></b>\n"),
                ("app/views/pages/_card.de.html.erb", "<%# locals: (name:) -%>\n<b><%= name %></b>\n"),
            ],
            "their `locals:` headers differ",
        ),
        (
            &[
                ("config/application.rb", "module Blog\n  class Application < Rails::Application\n    config.i18n.available_locales = ENV.fetch(\"LOCALES\").split\n  end\nend\n"),
            ],
            "which locales `I18n.locale` can be is unknown",
        ),
        (
            &[("config/environments/production.rb", "Rails.application.configure do\n  config.i18n.fallbacks = [:de]\nend\n")],
            "the template differs by environment",
        ),
    ];
    for (extra, want) in cases {
        let got = errors(extra);
        assert!(got.iter().any(|e| e.contains(want)), "{want:?} not among {got:#?}");
    }
}

/// A template no action renders reports its errors as warnings in every
/// locale's file, not only the default locale's.
#[test]
fn an_unreached_template_warns_in_each_variant() {
    let errors = app(false)
        .write("app/views/pages/orphan.en.html.erb", "<p><%= @missing.title %></p>\n")
        .write("app/views/pages/orphan.de.html.erb", "<p><%= @missing.name %></p>\n")
        .check_errors();
    assert_eq!(errors, Vec::<String>::new());
}

/// The Spinel tree renders each locale's template.
#[test]
#[ignore = "requires the Spinel toolchain"]
fn each_locale_renders_its_template_on_spinel() {
    app(false)
        .run_spinel(
            "I18n.locale = :de\nde = [Views::Pages.welcome(\"x\"), Views::Pages.about, Views::Pages.faq, Views::Layouts.application(\"b\")].join(\"|\")\n\
             I18n.locale = :en\nen = [Views::Pages.welcome(\"x\"), Views::Pages.about, Views::Pages.faq, Views::Layouts.application(\"b\")].join(\"|\")\n\
             got = (de + \"/\" + en).gsub(\"\\n\", \"\")\n\
             want = \"<h1>Willkommen x</h1>|<p>Ueber uns</p>|<p>FAQ in English</p>|[de]b/<h1>Welcome x</h1>|<p>About us</p>|<p>FAQ in English</p>|[en]b\"\n\
             if got == want\n  puts \"PASS\"\nelse\n  puts got\n  exit 1\nend\n",
        )
        .assert_passes();
}

/// The emitted Spinel tree has no locale in a view's name and reports nothing.
#[test]
fn the_spinel_tree_has_one_method_per_template() {
    let (emitted, errors) = app(false).emit(BuildTarget::Spinel);
    assert!(errors.is_empty(), "{errors:#?}");
    let pages = std::fs::read_to_string(emitted.join("app/views/pages/welcome.rb")).expect("welcome.rb");
    assert!(pages.contains("def self.welcome(") && pages.contains("case I18n.locale.to_s"), "{pages}");
    assert!(!emitted.join("app/views/pages/welcome.de.rb").exists());
}

fn config_rb(config: &str) -> String {
    format!("require_relative \"boot\"\nrequire \"rails/all\"\n\nmodule Blog\n  class Application < Rails::Application\n    config.load_defaults 8.1\n{config}  end\nend\n")
}

fn locale_template_errors(app: emit_and_run::Overlay) -> Vec<String> {
    app.check_errors().into_iter().filter(|e| e.contains("locale_template")).collect()
}

/// `available_locales` computed at boot, with only `en.yml` to read: the
/// locales it allows may include `de` (or a child of it), which would
/// render `welcome.de`, so that choice is an error even though only one
/// locale is known; `faq`, whose one locale template is the default
/// locale's, answers every locale alike and stays.
#[test]
fn an_unknown_locale_set_is_an_error_where_a_locale_template_depends_on_it() {
    let errors = locale_template_errors(
        app(false)
            .remove("config/locales/de.yml")
            .write("config/application.rb", &config_rb("    config.i18n.available_locales = ENV.fetch(\"LOCALES\", \"en de\").split.map(&:to_sym)\n")),
    );
    assert!(errors.iter().any(|e| e.contains("`pages/welcome`") && e.contains("whether `de` renders")), "{errors:#?}");
    assert!(!errors.iter().any(|e| e.contains("`pages/faq`")), "{errors:#?}");
}

/// With `available_locales` unset, Rails makes every locale its load path
/// names available, Rails' own `en` among them: an app with only `de.yml`
/// still renders `welcome.en` for `en` (`rails_makes_its_own_en_available`).
/// This build's locales are the app's files', so that template is an
/// error, not dropped. A locale no file names (`fr`) is never available,
/// and its template is never picked there either.
#[test]
fn an_unset_available_locales_is_an_error_for_a_locale_only_rails_files_make_available() {
    let errors = locale_template_errors(
        app(false)
            .remove("config/locales/en.yml")
            .write("config/application.rb", &config_rb("    config.i18n.default_locale = :de\n"))
            .write("app/views/pages/greeting.de.html.erb", "<p>Hallo</p>\n")
            .write("app/views/pages/greeting.fr.html.erb", "<p>Salut</p>\n"),
    );
    assert!(errors.iter().any(|e| e.contains("`pages/welcome`") && e.contains("makes `en` available")), "{errors:#?}");
    assert!(!errors.iter().any(|e| e.contains("`pages/greeting`")), "{errors:#?}");
}

/// The two tests above, against Rails 8.1.4: with only `de.yml` and no
/// `available_locales`, `en` renders `welcome.en` and `fr` is invalid.
#[test]
fn rails_makes_its_own_en_available() {
    let probe = r#"gem "railties", "8.1.4"; gem "activerecord", "8.1.4"; gem "actionmailer", "8.1.4"; require "rails"; require "sqlite3""#;
    if !Command::new("ruby").args(["-e", probe]).status().is_ok_and(|s| s.success()) {
        eprintln!("skipping: ruby with railties 8.1.4, actionmailer and sqlite3 not available");
        return;
    }
    let root = scratch::dir("roundhouse-locale-templates-unset");
    for (path, body) in files(false).into_iter().filter(|(p, _)| p != "config/locales/en.yml") {
        let p = root.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    let asserts = "def req(q)\n  status, _h, body = RefApp.call(Rack::MockRequest.env_for(\"/welcome?\" + q))\n  text = +\"\"\n  body.each { |part| text << part }\n  [status, text.gsub(/\\s*\\n\\s*/, \"\")]\nrescue I18n::InvalidLocale => e\n  e.class.name\nend\n\
        got = [req(\"locale=en\"), req(\"locale=fr\"), I18n.available_locales.sort]\n\
        want = [[200, \"[en]<h1>Welcome readers</h1>\"], \"I18n::InvalidLocale\", [:de, :en]]\n\
        raise got.inspect unless got == want\n";
    let output = rails(&root, "    config.i18n.default_locale = :de\n", asserts);
    let _ = std::fs::remove_dir_all(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("Rails agrees"),
        "=== stdout ===\n{stdout}\n=== stderr ===\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The table, against Rails 8.1.4.
#[test]
fn rails_answers_the_table() {
    let probe = r#"gem "railties", "8.1.4"; gem "activerecord", "8.1.4"; gem "actionmailer", "8.1.4"; require "rails"; require "sqlite3""#;
    if !Command::new("ruby").args(["-e", probe]).status().is_ok_and(|s| s.success()) {
        eprintln!("skipping: ruby with railties 8.1.4, actionmailer and sqlite3 not available");
        return;
    }
    for fallbacks in [false, true] {
        let root = scratch::dir("roundhouse-locale-templates");
        for (path, body) in files(fallbacks) {
            let p = root.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        let config = format!("{I18N}{}", if fallbacks { FALLBACKS } else { "" });
        let call = "RefApp.call(Rack::MockRequest.env_for(\"/\" + env[0] + \"?\" + env[1]))";
        let output = rails(&root, &config, &assertions(call, fallbacks));
        let _ = std::fs::remove_dir_all(&root);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("Rails agrees"),
            "Rails 8.1.4 disagrees with the table (fallbacks: {fallbacks})\n=== stdout ===\n{stdout}\n=== stderr ===\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn rails(root: &Path, config: &str, asserts: &str) -> std::process::Output {
    let script = format!(
        r#"gem "railties", "8.1.4"
gem "activerecord", "8.1.4"
gem "actionmailer", "8.1.4"
ENV["DATABASE_URL"] = "sqlite3::memory:"
require "rails"
require "active_record/railtie"
require "action_controller/railtie"
require "action_view/railtie"
require "action_mailer/railtie"
require "rack/mock"
class RefApp < Rails::Application
  config.root = {root:?}
  config.load_defaults 8.1
  config.eager_load = false
  config.secret_key_base = "x" * 64
  config.hosts.clear
  config.logger = Logger.new(nil)
  config.action_dispatch.show_exceptions = :none
  config.active_record.maintain_test_schema = false
  config.action_mailer.delivery_method = :test
{config}end
RefApp.initialize!
ActiveRecord::Schema.verbose = false
load File.join({root:?}, "db/schema.rb")
{asserts}puts "Rails agrees"
"#,
        root = root.display().to_string(),
    );
    Command::new("ruby").arg("-e").arg(&script).output().expect("ruby is on PATH")
}
