#[macro_use]
extern crate duct;
extern crate rusqlite;
extern crate time;

use duct::cmd;
use rusqlite::{Connection, Result, Error};
use time::Timespec;
use std::fs::File;
use std::io::{Read, Write};

#[derive(Debug)]
struct Course {
    course_id: i32,
    course_num: i32,
    term_id: i32,
    subject: String,
    url: String
}

// Is there a more ergonomic way of doing this?
macro_rules! course_row {
    ( $r:expr ) => { &[&$r.course_id, &$r.course_num, &$r.term_id, &$r.subject] }
}

impl Course {
    fn new<S: Into<String>>(course_id: i32, course_num: i32, term_id: i32, subject: S) -> Self {
        let string = subject.into();
        let url = format!("https://wl11gp.neu.edu/udcprod8/bwckschd.p_disp_listcrse?term_in={}&subj_in={}&crse_in={}&crn_in={}",
                            term_id,
                            &string,
                            course_num,
                            course_id);
        Course {
            course_id: course_id,
            course_num: course_num,
            term_id: term_id,
            subject: string,
            url: url
        }
    }

    fn check_open_seats(&self) -> std::result::Result<u32, String> {
        use std::error::Error;
        let curl_stdout = cmd("curl", &["-s", &self.url])
            .pipe(cmd!("sed '140q;d'"))
            .pipe(cmd!("sed -n -E 's/<TD CLASS=\"dddefault\">|<\\/TD>//gp'"))
            .read();
        curl_stdout.map_err(|e| e.description().into())
            .and_then(|out| out.parse::<u32>().map_err(|e| e.description().into()))
    }

}

fn main() {
    println!("Hello, world!");
}

fn get_courses(conn: &mut Connection) -> Result<Vec<Course>> {
    let mut select = conn.prepare("SELECT * FROM COURSES")?;
    let course_iter = select.query_map(&[], |row| {
        Course::new::<String>(row.get(0), row.get(1), row.get(2), row.get(3))
    })?;
    course_iter.collect::<Result<Vec<Course>>>()
}

fn create_courses(conn: &mut Connection, courses: &Vec<Course>) -> Result<()> {
    let trans = conn.transaction()?;
    { // Is this necessary?
        let mut insert = trans.prepare("INSERT INTO COURSES(course_id, course_num, term_id, subject) VALUES (?, ?, ?, ?);")?;
        for c in courses {
            insert.execute(course_row!(c))?;
        }
    }
    trans.commit();
    Ok(())
}

fn execute_file(conn: &mut Connection, file_path: &str) -> Result<()> {
    let file_try = File::open(file_path);
    if let Err(e) = file_try {
        writeln!(std::io::stderr(), "Error while trying to open file `{}`: {}", file_path, e);
        return Err(Error::InvalidPath(file_path.into()));
    }
    let mut file = file_try.unwrap();
    let mut buf = String::new();
    if let Err(e) = file.read_to_string(&mut buf) {
        writeln!(std::io::stderr(), "Error while reading file `{}`: {}", file_path, e);
        return Err(Error::InvalidPath(file_path.into()));
    }
    conn.execute_batch(&buf)
}

#[test]
fn test_courses() {
    let mut conn = Connection::open_in_memory().unwrap();
    execute_file(&mut conn, "create_tables.sql").expect("Could not create tables");
    execute_file(&mut conn, "insert_engw_courses.sql").expect("Could not insert ENGW courses");
    // let created = create_courses(&mut conn, &vec![
    //                              Course::new(1, 2, 3, "FOO"),
    //                              Course::new(2, 3, 4, "FOO"),
    //                              Course::new(3, 4, 5, "FOO"),
    //                              Course::new(4, 5, 6, "BAZ")])
    //     .expect("Could not create courses");
    let courses = get_courses(&mut conn).expect("Could not get courses from db");
    // println!("{:?}", &courses);
    println!("{}", &courses[0].url);
    println!("{:?}", courses[0].check_open_seats());
}
