use chrono::DateTime;
use chrono::{Date, Local, Utc};
use colored::Colorize;
use regex::Regex;
use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;
use std::{
    env,
    fs::{self, DirEntry, ReadDir},
    os,
    time::SystemTime,
};
fn print<T: std::fmt::Display>(s: T) {
    println!("{}", s);
}

#[derive(Debug)]
enum FSElement {
    Directory(Vec<FSElement>,String),
    File {
        name: String,
        path: Rc<PathBuf>,
        is_md: bool,
    },
}



impl FSElement {
    fn sort_value(&self)->i8{
        match self {
            FSElement::Directory(_, _) => 1,
            FSElement::File { name, path, is_md } => 0,
        }
    }

    fn is_dir(&self) -> bool{
        match self {
            FSElement::Directory(_, _) => true,
            FSElement::File { name, path, is_md } => false,
        }
    }

    fn to_md(&self, dir_only: bool) -> String{
        let mut s = String::with_capacity(100);
        match self {
            FSElement::Directory(d,name) => {
                s.push_str(format!("📁{}  \n",name).as_str());
                
                for (index, element) in d.iter().enumerate(){
                    if dir_only && !element.is_dir(){
                        continue;
                    }
                    let char;
                    if index == d.len()-1{
                        char = '└'
                    }else{
                        char = '├'
                    }
                    s.push_str(format!("{}─{}  \n",char,element.to_md(dir_only)).as_str());
                }
                s = s.lines().collect::<Vec<&str>>().join("  \n│&emsp;");
                s
            },
            FSElement::File { name, path, is_md } => {
                let path = path.as_os_str();
                s.push_str(format!("[📄{}](<{}>)",name,path.to_str().unwrap().replace("\\", "/")).as_str());
                s
            },
        }
    }

    fn get_markdowns(&self) -> Vec<FSElement> {
        let mut v = Vec::with_capacity(20);
        fn get_markdowns_rec(v: &mut Vec<FSElement>, root: &Vec<FSElement>) {
            for e in root {
                match e {
                    FSElement::Directory(d,name) => get_markdowns_rec(v, d),
                    FSElement::File { name, path, is_md } => {
                        if *is_md {
                            v.push(FSElement::File {
                                name: (name.clone()),
                                path: path.clone(),
                                is_md: *is_md,
                            })
                        }
                    }
                }
            }
        }
        if let FSElement::Directory(d,name) = self{
            get_markdowns_rec(&mut v, d);
        }
        return v;
    }
}

fn index_filesystem(dir: ReadDir, forbidden_paths: &Vec<String>, root_fs: &mut FSElement) {
    let root_vec;
    if let FSElement::Directory(v,name) = root_fs{
        root_vec = v
    }else{
        panic!("Error: root_fs element is not directory");
    }
    for path in dir {
        if path.is_err() {
            continue;
        }

        let path = path.unwrap();
        let name = path.file_name();
        let name = name.to_str().unwrap();
        if forbidden_paths.iter().find(|e| e.contains(name)).is_some() {
            continue;
        }
        let t = path.file_type().unwrap();
        if t.is_dir() {
            let read_dir = fs::read_dir(path.path());
            if let Err(e) = read_dir {
                print(e);
                continue;
            }
            let read_dir = read_dir.unwrap();
            let mut fs_directory = FSElement::Directory(Vec::with_capacity(10),name.to_owned());
            index_filesystem(read_dir, &forbidden_paths, &mut fs_directory);
            if let FSElement::Directory(d, name) = &mut fs_directory{
                d.sort_by(|a,b|{
                    a.sort_value().cmp(&b.sort_value())
                });
            }
            root_vec.push(fs_directory);
        } else {
            root_vec.push(FSElement::File {
                name: name.to_owned(),
                path: Rc::new(path.path()),
                is_md: name.ends_with(".md"),
            });
        }
    }
}

#[derive(Debug)]
struct HeadLine<'a> {
    intend: u8,
    title: &'a str,
}

impl HeadLine<'_> {
    fn to_md(&self) -> String {
        return format!(
            "{}- [{}](#{})  \n",
            "    ".repeat((self.intend - 1).into()),
            self.title,
            self.title.to_ascii_lowercase().replace(" ", "-")
        );
    }
}

fn process_md(path: PathBuf, name: &str) {
    let content = fs::read_to_string(path.clone());
    if let Err(e) = content {
        println!("{}", e);
        return;
    }
    let mut content = content.unwrap();

    let mut head_lines = Vec::<HeadLine>::new();

    for line in content.lines() {
        if line.starts_with("#") {
            let (hrcount, title) = line.split_once(" ").unwrap();
            head_lines.push(HeadLine {
                intend: hrcount.len() as u8,
                title,
            })
        }
    }

    let mut h1 = 0;
    head_lines.iter().for_each(|e| {
        if e.intend == 1 {
            h1 += 1;
        }
    });
    if h1 == 1 {
        head_lines.drain(0..1);
        head_lines.iter_mut().for_each(|e| {
            e.intend -= 1;
        })
    }

    let mut result = String::with_capacity(head_lines.len() * 20);
    result.push_str(TOC_BEGIN_PREFIX);
    result.push('\n');
    head_lines
        .iter()
        .for_each(|h| result.push_str(h.to_md().as_str()));

    result.push_str(
        format!(
            "<sup><sup>Last update: {}</sup></sup>\n",
            Local::now().format("%d.%m.%Y %H:%M")
        )
        .as_str(),
    );
    result.push_str(TOC_END_PREFIX);

    let rep;
    if content.contains(TOC_FIRST_PREFIX) {
        rep = content.replace(TOC_FIRST_PREFIX, &result.as_str());
    } else {
        let re_str = format!(r"{}([\S\s]*?){}", TOC_BEGIN_PREFIX, TOC_END_PREFIX);
        let re: Regex = Regex::new(re_str.as_str()).unwrap();
        rep = re.replace(&content, result.as_str()).to_string();
    }
    if rep != content {
        let res = fs::write(path.clone(), rep);
        if let Err(e) = res {
            println!("ERROR updating {} - {}", name.red(), e.to_string().red());
        } else {
            println!("{} updated sucessfully!", name.green());
        }
    }
}
const TOC_FIRST_PREFIX: &str = "<!--%toc%-->";
const TOC_BEGIN_PREFIX: &str = "<!--%table_of_contents_begin%-->";
const TOC_END_PREFIX: &str = "<!--%table_of_contents_end%-->";

const GFS_FIRST_PREFIX: &str = "<!--%gfs%-->";
const GFS_BEGIN_PREFIX: &str = "<!--%file_structure_begin%-->";
const GFS_END_PREFIX: &str = "<!--%file_structure_end%-->";
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let dir = fs::read_dir("./").unwrap();
    let gitignore = fs::read_to_string("./.gitignore");

    let mut forbidden_paths = if let Ok(s) = gitignore {
        let lines = s.lines();
        lines.map(String::from).collect()
    } else {
        Vec::new()
    };
    forbidden_paths.push(".git".to_owned());


    let mut indexed_fs = FSElement::Directory(Vec::new(),"./".to_owned());
    index_filesystem(dir, &forbidden_paths, &mut indexed_fs);
    let mut args_iter = args.iter();
    let key = args_iter.next();
    
    if let Some(mut k) = key {
        let k = k.to_ascii_lowercase();
        match k.as_str() {
            "gtoc" => {
                indexed_fs.get_markdowns().iter().for_each(|m|{
                    if let FSElement::File { name, path, is_md: _ } = m{
                        process_md((*path).to_path_buf(), name.as_str())
                    }
                })
                // process_dir(dir, &forbidden_paths)
            }
            "gfs" => {
                let arg1 = args_iter.next();
                let mut dir_only = false;
                if let Some(v) = arg1{
                    if v.contains("--dironly"){
                        println!("dironly=true");
                        dir_only = true;
                    }
                }
                process_md_fs(&indexed_fs, dir_only)
            }
            _ => {
                println!("{}Unknown arg '{}'", "[ERROR]: ".bold().red(), k.blue())
            }
        };
    } else {
        println!(
            "
{}
{}
[COMMANDS]
gtoc            | Embed '{}' in your markdown document to generate a table of content
gfs [--dironly] | Embed '{}' in your markdown doc to generate a view of subdirectories

",
            "===MarkdownUtils===".bold().green(),
            "by Jadr".blue(),
            TOC_FIRST_PREFIX.blue(),
            GFS_FIRST_PREFIX.blue()
        )
    }
}

fn process_md_fs(e: &FSElement, dir_only: bool){
    fn process_md(root: &FSElement, mdfile: &FSElement, dir_only: bool){
        if let FSElement::File { name, path, is_md} = mdfile{
            
            let content = fs::read_to_string(&**path);
            if let Err(e) = content {
                println!("Err{}", e);
                return;
            }

            fn pre_suffix(md: String) -> String{
                let mut s = String::with_capacity(md.len()+GFS_BEGIN_PREFIX.len()+GFS_END_PREFIX.len());
                s.push_str(GFS_BEGIN_PREFIX);
                s.push('\n');
                s.push_str(md.as_str());
                s.push('\n');
                s.push_str(GFS_END_PREFIX);
                s.push('\n');
                return s
            }
            
            
            let content = content.unwrap();
            let embed;
            if content.contains(GFS_FIRST_PREFIX){
                embed = content.replace(GFS_FIRST_PREFIX, &pre_suffix(root.to_md(dir_only)));
            } else {
                let re_str = format!(r"{}([\S\s]*?){}", GFS_BEGIN_PREFIX, GFS_END_PREFIX);
                let re: Regex = Regex::new(re_str.as_str()).unwrap();
                embed = re.replace(&content, &pre_suffix(root.to_md(dir_only))).to_string();
            }

            if embed != content {
                let res = fs::write((**path).clone(), embed);
                if let Err(e) = res {
                    println!("ERROR updating {} - {}", name.red(), e.to_string().red());
                } else {
                    println!("{} updated sucessfully!", name.green());
                }
            }
            
        }
    }

    match e {
        FSElement::Directory(vec, _) => {
            for element in vec{
                if let FSElement::File { name:_, path: _, is_md} = element{
                    if *is_md{
                        process_md(e, element,dir_only)
                    }
                }else{
                    process_md_fs(element,dir_only);
                }
            }
        },
        FSElement::File { name, path, is_md } => {
            panic!()
        },
    }
}
