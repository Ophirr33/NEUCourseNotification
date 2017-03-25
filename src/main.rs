extern crate chrono;
extern crate clap;
#[macro_use]
extern crate duct;
extern crate lettre;
extern crate rusqlite;

use clap::{Arg, App};
use chrono::UTC;
use rusqlite::{Connection, Result, Error};
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

    // Curls the course website and checks the open seats section from the html
    fn check_open_seats(&self) -> std::result::Result<i32, String> {
        use std::error::Error;
        let curl_stdout = cmd!("curl", "-s", &self.url)
            .pipe(cmd!("sed", "140q;d"))
            .pipe(cmd!("sed", "-n", "-E", "s/<TD CLASS=\"dddefault\">|<\\/TD>//gp"))
            .read();
        curl_stdout.map_err(|e| e.description().into())
            .and_then(|out| out.parse::<i32>().map_err(|e| e.description().into()))
    }

    // Parses the result from check_open_seats, and sends an email about it
    fn email_result(&self, count: std::result::Result<i32, String>, recipient: &str) -> std::io::Result<String> {
        let email_message = match count {
            Ok(n) if n == 0 => format!("Unfortunately, there are still no slots open for course {}.\
                                       \nCourse URL: {}",
                                       self.course_id,
                                       &self.url),
            Ok(n) if n == 1 => format!("Hurry! There's an open spot available for course {}.\
                                       \nCourse URL: {}",
                                       self.course_id,
                                       &self.url),
            Ok(n) => format!("Congrats! There are {} open spots for course {}.\nCourse URL: {}",
                             n,
                             self.course_id,
                             &self.url),
            Err(mes) => format!("Something went wrong when looking at open seats for course {}.\
                                Failed with output `{}`.\nCourse URL: {}",
                                mes,
                                self.course_id,
                                &self.url)
        };
        cmd!("echo", email_message)
            .pipe(cmd!("msmtp", "-a", "default", recipient))
            .read()
    }

    // Just stores the count into the database for the 4 hour reports
    fn persist_count(&self, count: i32, dao: &mut DAO) -> Result<()> {
        dao.insert_scraping_count(count, self.course_id)
            .map(|_| ())
    }
}

struct DAO {
    conn: Connection
}

impl DAO {
    fn new(conn: Connection) -> Self {
        DAO { conn: conn }
    }

    fn get_courses(&self) -> Result<Vec<Course>> {
        let mut select = self.conn.prepare("SELECT * FROM COURSES")?;
        let course_iter = select.query_map(&[], |row| {
            Course::new::<String>(row.get(0), row.get(1), row.get(2), row.get(3))
        })?;
        course_iter.collect::<Result<Vec<Course>>>()
    }

    #[allow(dead_code)]
    fn create_courses(&mut self, courses: &Vec<Course>) -> Result<()> {
        let trans = self.conn.transaction()?;
        { // Is this necessary?
            let mut insert = trans.prepare("INSERT INTO COURSES(course_id, course_num, term_id, subject) VALUES (?, ?, ?, ?);")?;
            for c in courses {
                insert.execute(course_row!(c))?;
            }
        }
        trans.commit()?;
        Ok(())
    }

    fn insert_scraping_count(&mut self, count: i32, course: i32) -> Result<i32> {
        let mut insert = self.conn.prepare("INSERT INTO SCRAPES(count, course) VALUES (?, ?);")?;
        insert.execute(&[&count, &course])
    }

    fn build_report(&self) -> Result<Vec<(i32, i32, i32)>> {
        let mut select = self.conn.prepare(
            "SELECT course_id, count, COUNT(*) \
            FROM COURSES JOIN SCRAPES ON SCRAPES.course = COURSES.course_id \
            WHERE SCRAPES.timestamp > datetime('now', '-4 hours', '-5 minutes') \
            GROUP BY SCRAPES.count, COURSES.course_id;")?;
        let report_iter = select.query_map(&[], |row| {
            (row.get(0), row.get(1), row.get(2))
        })?;
        report_iter.collect::<Result<Vec<(i32, i32, i32)>>>()
    }

    fn execute_file(&self, file_path: &str) -> Result<()> {
        let file_try = File::open(file_path);
        if let Err(e) = file_try {
            writeln!(std::io::stderr(), "Error while trying to open file `{}`: {}", file_path, e)
                .expect("Could not write line to standard error");
            return Err(Error::InvalidPath(file_path.into()));
        }
        let mut file = file_try.unwrap();
        let mut buf = String::new();
        if let Err(e) = file.read_to_string(&mut buf) {
            writeln!(std::io::stderr(), "Error while reading file `{}`: {}", file_path, e)
                .expect("Could not write line to stderr");
            return Err(Error::InvalidPath(file_path.into()));
        } else {
            self.conn.execute_batch(&buf)
        }
    }
}

fn email_report(dao: &mut DAO, recipient: &str) -> std::result::Result<String, String> {
    use std::error::Error;
    let report_vec = dao.build_report();
    let email_base = format!("Subject: 4 Hour Report - {}\n\n\
                      Course ID | Spots Open | Occurrences\n",
                     UTC::now().format("%D %H"));
    report_vec.map::<String, _>(|v| v.into_iter()
                                .fold(email_base.into(),
                                |acc, row| acc + &format!("{:<11} | {:<20} | {}\n", row.0, row.1, row.2)))
        .map_err(|e| e.description().into())
        .and_then(|report|
                  cmd!("echo", report)
                  .pipe(cmd!("msmtp", "-a", "default", recipient))
                  .read()
                  .map_err(|e| e.description().into()))
}

fn main() {
    let matches = App::new("Neu Course Open Seat Notifcations")
        .version("0.1")
        .about("Chekcs MyNeu and sees if a course is open. Builds summarazing reports")
        .author("Ty Coghlan")
        .arg(Arg::with_name("database file")
             .help("The sqlite3 database to use")
             .long("database-file")
             .short("f")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name("initialize")
             .help("Initializes the database with the given file")
             .long("initialize")
             .short("i")
             .takes_value(true)
             .required(false))
        .arg(Arg::with_name("recipient")
             .help("Who's getting the emails")
             .long("recipient")
             .short("r")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name("build report")
             .help("Send report from last 4 hours")
             .long("build-report")
             .short("b")
             .takes_value(false)
             .required(false))
        .arg(Arg::with_name("open seats")
             .help("Send an email if this many of open seats are available")
             .long("open-seats")
             .short("o")
             .takes_value(true)
             .required(false))
        .get_matches();
    let mut dao = DAO::new(Connection::open(matches.value_of("database file").unwrap()).unwrap());
    let init_file = matches.value_of("initialize");
    let recipient = matches.value_of("recipient").unwrap();
    let build_report = matches.is_present("build report");
    let open_seats = matches.value_of("open_seats") .unwrap_or("1")
        .parse::<i32>()
        .unwrap_or(1);
    if let Some(f) = init_file {
        dao.execute_file(f).expect("Could not execute initialization file!");
    }
    let courses = dao.get_courses().expect("Could not load courses from database!");
    for course in courses.into_iter() {
        let seats_result = course.check_open_seats();
        match seats_result.clone() {
            Ok(n) if n >= open_seats => {
                course.email_result(seats_result, recipient).expect("Error sending email!");
                course.persist_count(n, &mut dao).expect("Error storing scrape result into database!");
            },
            Ok(n) => { course.persist_count(n, &mut dao).expect("Error storing scrape result into database!"); },
            Err(_) => { course.email_result(seats_result, recipient).expect("Error sending email!"); }
        }
    }
    if build_report {
        email_report(&mut dao, recipient).expect("Error sending email!");
    }
}

#[test]
fn test_courses() {
    let mut dao = DAO::new(Connection::open_in_memory().unwrap());
    dao.execute_file("create_tables.sql").expect("Could not create tables");
    dao.execute_file("insert_engw_courses.sql").expect("Could not insert ENGW courses");
    dao.execute_file("insert_scrapes.sql").expect("Could not insert scrape data");
    let courses = dao.get_courses().expect("Could not get courses from db");
    assert_eq!(courses.len(), 6);
    assert_eq!(courses[0].course_id, 60356);
    assert_eq!(courses[0].course_num, 3302);
    assert_eq!(courses[0].term_id, 201760);
    assert_eq!(courses[0].subject, "ENGW");
    let open_seats = courses[0].check_open_seats();
    assert_eq!(open_seats, Ok(0));
    assert_eq!(vec![(60356, 0, 3), (60415, 0, 1), (60418, 0, 1), (60356, 1, 1), (60415, 2, 1)],
               dao.build_report().expect("Could not build report"));
}
