extern crate cli_agenda;
extern crate glob;
extern crate regex;
extern crate reqwest;
extern crate rusqlite;
extern crate scraper;

use std::collections::{HashMap, HashSet};
use std::fs;

const BASE: &str = "http://bloodontheclocktower.com";
const KEYWORDS: &[&str] = &[
    "townsfolk",
    "outsider",
    "minion",
    "demon",
    "drunk",
    "sober",
    "poison",
    "red herring",
    "nominate",
    "nomination",
    "execute",
    "execution",
    "alive",
    "vote",
    "register",
    "storyteller",
    "evil",
    "good",
    "win",
    "die",
    "night",
    "day",
    "1st",
    "final",
    "first",
    "private",
    "game end",
    "false",
    "twins",
    "good twin",
    "traveler",
    "exile",
];

fn main() {
    let headlines = scraper::Selector::parse(".large-block-grid-4 .mw-headline").unwrap();
    let link = scraper::Selector::parse("a").unwrap();
    let big_boxes = scraper::Selector::parse(".row .panel.large-centered").unwrap();
    let para = scraper::Selector::parse("p").unwrap();
    let simplify = regex::Regex::new(r#"[^A-Za-z0-9]"#).unwrap();

    let db = rusqlite::Connection::open("../.data/sqlite.db").unwrap();
    db.execute_batch("DELETE FROM taggings").unwrap();
    let mut findcase = db
        .prepare("SELECT id FROM cases WHERE explanation = ?")
        .unwrap();
    let mut findtag = db.prepare("SELECT id FROM tags WHERE tag = ?").unwrap();
    let mut mktag = db.prepare("INSERT INTO tags (tag) VALUES (?)").unwrap();
    let mut mkexmpl = db
        .prepare("INSERT INTO cases (explanation) VALUES (?)")
        .unwrap();
    let mut bind = db
        .prepare("INSERT INTO taggings (tag_id, case_id) VALUES (?, ?)")
        .unwrap();

    let mut log = cli_agenda::start();
    let mut pages = Vec::new();
    let mut characters = HashMap::new();

    // first, find all characters
    log = log.enter("Searching wiki for characters");
    for edition in &["Trouble_Brewing", "Sects_%26_Violets", "Bad_Moon_Rising"] {
        let url = format!("{}/wiki/{}", BASE, edition);
        let text = reqwest::get(&url).unwrap().text().unwrap();
        let page = scraper::Html::parse_document(&text);
        pages.push((edition, page));
    }

    for (edition, page) in &pages {
        log = log.enter(edition);
        for maybe_character in page.select(&headlines) {
            match maybe_character.value().attr("id") {
                None => continue,
                Some(id) => {
                    let mut chars = id.chars();
                    match chars.next() {
                        None => continue,
                        Some(ch) if !ch.is_uppercase() => continue,
                        _ => {}
                    }
                }
            }

            match maybe_character.select(&link).next() {
                Some(a) => {
                    let name = a.text().fold(String::new(), |mut name, s| {
                        name.push_str(s);
                        name
                    });
                    let url = match a.value().attr("href") {
                        Some(href) => href,
                        None => continue,
                    };

                    log.single(format!("found {}", name));
                    characters.insert(name, url);
                }
                None => continue,
            }
        }
        log = log.leave();
    }
    log = log.leave();

    log = log.enter("Establishing character matching");
    let all_of_interest = characters
        .keys()
        .map(|s| &**s)
        .chain(KEYWORDS.into_iter().map(|s| &**s))
        .fold(String::new(), |mut acc, s| {
            if acc.is_empty() {
                String::from(s)
            } else {
                acc.push_str("|");
                acc.push_str(&*simplify.replace_all(s, "[^0-9a-zA-Z]"));
                acc
            }
        });
    let all_of_interest = format!(r#"\b({})(s|e?d|ly)?\b"#, all_of_interest);
    println!("\n{}", all_of_interest);
    let all_of_interest = regex::RegexBuilder::new(&all_of_interest)
        .case_insensitive(true)
        .build()
        .unwrap();
    log = log.leave();

    let mut incorporate_example = |log: &mut cli_agenda::Progress, text: &str| {
        let text = text.trim();
        let tags: HashSet<_> = all_of_interest
            .captures_iter(&text)
            .map(|cap| {
                simplify
                    .replace_all(&cap[1], regex::NoExpand(""))
                    .into_owned()
                    .to_lowercase()
            })
            .collect();

        *log = log.enter("found example");
        eprintln!("\n{}\n", text);
        let example = match findcase.query_row(&[&text], |row| row.get::<_, i64>(0)) {
            Ok(id) => {
                log.warn(format!("already exists as {}", id));
                id
            }
            Err(_) => mkexmpl.insert(&[&text]).unwrap(),
        };

        for tag in tags {
            let tag_id = if let Ok(id) = findtag.query_row(&[&tag], |row| row.get::<_, i64>(0)) {
                id
            } else {
                mktag.insert(&[&tag]).unwrap()
            };
            log.single(format!("tagged with '{}' ({})", tag, tag_id));
            bind.execute(&[&tag_id as &_, &example as &_]).unwrap();
        }
        *log = log.leave();
    };

    for (character, page) in &characters {
        let text = reqwest::get(&format!("{}{}", BASE, page))
            .unwrap()
            .text()
            .unwrap();
        let page = scraper::Html::parse_document(&text);

        log = log.enter(character);
        for maybe_example in page.select(&big_boxes) {
            maybe_example
                .select(&para)
                .map(|p| {
                    let text = p.text().fold(String::new(), |mut name, s| {
                        name.push_str(s);
                        name
                    });
                    incorporate_example(&mut log, &*text);
                })
                .count();
        }
        log = log.leave();
    }

    for file in glob::glob("../examples/*.txt").unwrap() {
        let file = file.unwrap();
        log = log.enter(file.display());
        let examples = fs::read_to_string(file).unwrap();
        for example in examples.split("\n\n\n") {
            if example.is_empty() {
                continue;
            }

            incorporate_example(&mut log, example);
        }
        log = log.leave();
    }
}
