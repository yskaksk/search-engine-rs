use std::env;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::collections::{HashMap, BTreeSet};

use serde::{Serialize, Deserialize};

const N: usize = 2;

#[derive(Serialize, Deserialize, Debug)]
struct Doc {
    id: u8,
    title: String,
    author: String,
    content: Vec<String>,
    raw_content: String
}

impl Doc {
    fn show(&self) {
        println!("  id:      {}", self.id);
        println!("  title:   {}", self.title);
        println!("  author:  {}", self.author);
        println!("  content: {}...", substring(&self.raw_content, 0, 25));
        println!("----------");
    }
}

#[derive(Deserialize, Debug)]
struct RawDoc {
    id: u8,
    title: String,
    author: String,
    content: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Documents {
    docs: Vec<Doc>
}

impl Documents {
    fn show(&self, ids: Vec<u8>) {
        for i in ids.iter() {
            let doc: &_ = self.docs.get((*i - 1) as usize).unwrap();
            doc.show();
        }
    }

    fn from_file(path: &str) -> Self {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        return serde_json::from_reader(reader).unwrap();
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct InversedIndex {
    map: HashMap<String, BTreeSet<u8>>
}

impl InversedIndex {
    fn new() -> Self {
        InversedIndex {
            map: HashMap::new(),
        }
    }

    fn add(&mut self, keyword: &String, id: u8) {
        match self.map.get_mut(keyword) {
            Some(li) => {
                li.insert(id);
            },
            None => {
                let li: BTreeSet<u8> = BTreeSet::from([id]);
                self.map.insert(keyword.clone(), li);
            }
        }
    }

    fn build(&mut self, documents: &Documents) {
        for doc in documents.docs.iter() {
            self.add(&doc.title, doc.id);
            self.add(&doc.author, doc.id);
            for word in doc.content.iter() {
                self.add(word, doc.id);
            }
        }
    }

    fn save(&self) {
        let s = serde_json::to_string(self).unwrap();
        let file = File::create("file/index.json").unwrap();
        let mut writer = BufWriter::new(file);
        writer.write_all(s.as_bytes()).unwrap();
        println!("saved file/index.json");
    }

    fn from_file(path: &str) -> Self {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let index: Self = serde_json::from_reader(reader).unwrap();
        return index
    }

    fn search(&self, words: Vec<String>) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();

        let words: Vec<String> = words.iter().map(|w| tokenize(N, w.clone())).flatten().collect();
        let n: usize = words.len();
        if n == 1 {
            return match self.map.get(&words[0]) {
                Some(li) => {
                    li.iter().cloned().collect::<Vec<_>>()
                },
                None => result
            }
        }

        if words.iter().any(|w| self.map.get(w).is_none()) {
            return result
        }
        let mut table: Vec<_> = words.iter().map(|w| {
            self.map.get(w).unwrap().clone()
        }).collect();

        let mut ids: Vec<u8> = Vec::new();
        let mut min: Option<u8> = None;

        loop {
            let mut is_break = false;
            for i in 0..n {
                let li: &mut _ = table.get_mut(i).unwrap();
                let id = ids.get(i);
                match id {
                    Some(id) => {
                        match min {
                            Some(m) if *id == m => {
                                match li.pop_first() {
                                    Some(id) => ids[i] = id,
                                    None => {
                                        is_break = true;
                                        break
                                    }
                                }
                            },
                            _ => {},
                        }
                    },
                    None => {
                        match li.pop_first() {
                            Some(id) => ids.push(id),
                            None => is_break = true
                        }
                    }
                }
            }
            if is_break {
                break
            }
            min = ids.iter().min().copied();
            if ids.iter().all(|&id| id == min.unwrap()) {
                result.push(min.unwrap());
            }
        }
        return result
    }
}

fn generate_tokenized_doc() {
    let file = File::open("file/sample_text.csv").unwrap();
    let mut rdr = csv::Reader::from_reader(file);
    let mut docs = Vec::new();
    for r in rdr.deserialize() {
        let raw_doc: RawDoc = r.unwrap();
        let content = tokenize(N, String::from(format!("{}{}{}", raw_doc.title, raw_doc.author, raw_doc.content)));
        docs.push(Doc {
            id: raw_doc.id,
            title: raw_doc.title,
            author: raw_doc.author,
            content,
            raw_content: raw_doc.content
        });
    }
    let documents = Documents { docs };

    let file = File::create("file/documents.json").unwrap();
    let mut writer = BufWriter::new(file);
    let s = serde_json::to_string(&documents).unwrap();
    writer.write_all(s.as_bytes()).unwrap();

    println!("generated file/documents.json");
}

fn substring(s: &str, start: usize, length: usize) -> &str {
    if length == 0 {
        return "";
    }

    let mut ci = s.char_indices();
    let byte_start = match ci.nth(start) {
        Some(x) => x.0,
        None => return ""
    };

    match ci.nth(length - 1) {
        Some(x) => &s[byte_start..x.0],
        None => &s[byte_start..],
    }

}

fn tokenize(n: usize, content: String) -> Vec<String> {
    let len = content.chars().count();
    let mut result = Vec::new();
    if len <= n {
        result.push(content);
        return result
    }
    for i in 0..len-n {
        let ngram = String::from(substring(&content, i, n));
        result.push(ngram);
    }
    return result
}

fn generate_index() {
    let documents = Documents::from_file("file/documents.json");
    let mut ii = InversedIndex::new();
    ii.build(&documents);
    ii.save();
}

fn search() {
    let index = InversedIndex::from_file("file/index.json");
    let documents = Documents::from_file("file/documents.json");
    loop {
        println!("");
        println!("検索する単語をスペース区切りで入力してください");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("読み取れませんでした");
        input = input.trim_end_matches('\n').to_string();
        let words: Vec<String> = input.split(' ').map(|w| w.to_string()).collect();
        let searched = index.search(words);
        println!("{} result(s) found", searched.len());
        documents.show(searched);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("no command, do nothing");
        std::process::exit(0);
    }
    match &args[1][..] {
        "tokenize" => {
            generate_tokenized_doc();
        },
        "index" => {
            generate_index();
        },
        "search" => {
            search();
        },
        &_ => {},
    }
}
