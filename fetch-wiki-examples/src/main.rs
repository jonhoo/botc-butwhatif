extern crate regex;
extern crate reqwest;
extern crate rusqlite;
extern crate scraper;

use std::collections::{HashMap, HashSet};

const BASE: &str = "http://bloodontheclocktower.com";
const KEYWORDS: &[&str] = &[
    "townsfolk",
    "outsider",
    "minion",
    "demon",
    "drunk",
    "poison",
    "red herring",
    "nominate",
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
];

fn main() {
    let headlines = scraper::Selector::parse(".large-block-grid-4 .mw-headline").unwrap();
    let link = scraper::Selector::parse("a").unwrap();
    let big_boxes = scraper::Selector::parse(".row .panel.large-centered").unwrap();
    let para = scraper::Selector::parse("p").unwrap();
    let simplify = regex::Regex::new(r#"[^A-Za-z0-9]"#).unwrap();

    let db = rusqlite::Connection::open("../.data/sqlite.db").unwrap();
    db.execute_batch(
        "DELETE FROM taggings; \
         DELETE FROM tags; \
         DELETE FROM cases",
    ).unwrap();
    let mut findtag = db.prepare("SELECT id FROM tags WHERE tag = ?").unwrap();
    let mut mktag = db.prepare("INSERT INTO tags (tag) VALUES (?)").unwrap();
    let mut mkexmpl = db.prepare("INSERT INTO cases (explanation) VALUES (?)")
        .unwrap();
    let mut bind = db.prepare("INSERT INTO taggings (tag_id, case_id) VALUES (?, ?)")
        .unwrap();

    for edition in &["Trouble_Brewing"] {
        let url = format!("{}/wiki/{}", BASE, edition);
        let text = reqwest::get(&url).unwrap().text().unwrap();
        let page = scraper::Html::parse_document(&text);

        let mut characters = HashMap::new();
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

                    println!("==> found {} in {}", name, edition);
                    characters.insert(name, url);
                }
                None => continue,
            }
        }

        let all_of_interest = characters
            .keys()
            .map(|s| &**s)
            .chain(KEYWORDS.into_iter().map(|s| &**s))
            .fold(String::new(), |mut acc, s| {
                if acc.is_empty() {
                    String::from(s)
                } else {
                    acc.push_str("|");
                    acc.push_str(&s);
                    acc
                }
            });
        let all_of_interest = format!(r#"\b({})(s|ed)?\b"#, all_of_interest);
        println!("{}", all_of_interest);
        let all_of_interest = regex::RegexBuilder::new(&all_of_interest)
            .case_insensitive(true)
            .build()
            .unwrap();

        for page in characters.values() {
            let text = reqwest::get(&format!("{}{}", BASE, page))
                .unwrap()
                .text()
                .unwrap();
            let page = scraper::Html::parse_document(&text);

            for maybe_example in page.select(&big_boxes) {
                maybe_example
                    .select(&para)
                    .map(|p| {
                        let text = p.text().fold(String::new(), |mut name, s| {
                            name.push_str(s);
                            name
                        });
                        let tags: HashSet<_> = all_of_interest
                            .captures_iter(&text)
                            .map(|cap| {
                                simplify
                                    .replace_all(&cap[1], regex::NoExpand(""))
                                    .into_owned()
                                    .to_lowercase()
                            })
                            .collect();

                        println!("==> found example:\n\t{}", text);
                        let example = mkexmpl.insert(&[&text]).unwrap();
                        for tag in tags {
                            let tag_id = if let Ok(id) =
                                findtag.query_row(&[&tag], |row| row.get::<_, i64>(0))
                            {
                                id
                            } else {
                                mktag.insert(&[&tag]).unwrap()
                            };
                            println!(" -> tagged with '{}' ({})", tag, tag_id);
                            bind.execute(&[&tag_id as &_, &example as &_]).unwrap();
                        }
                    })
                    .count();
            }
        }
    }
}
