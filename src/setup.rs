use chrono::{expect, Datelike, Local};
use dirs::download_dir;
use promkit::{preset::QuerySelect, preset::Readline, preset::Select};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Course {
    pub name: String,
    pub description: String,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub download_path: PathBuf,
    pub base_path: PathBuf,
    pub current_term: String,
    pub start_year: i32,
    pub end_year: i32,
    pub coop: bool,
    pub courses: Vec<Course>,
    api_key: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct CourseInfo {
    courseId: Option<String>,
    courseOfferNumber: Option<i32>,
    termCode: Option<String>,
    termName: Option<String>,
    associatedAcademicCareer: Option<String>,
    associatedAcademicGroupCode: Option<String>,
    associatedCcademicOrgCode: Option<String>,
    subjectCode: Option<String>,
    catalogNumber: Option<String>,
    title: Option<String>,
    descriptionAbbreviated: Option<String>,
    description: Option<String>,
    gradingBasis: Option<String>,
    courseComponentCode: Option<String>,
    enrollConsentCode: Option<String>,
    enrollConsentDescription: Option<String>,
    dropConsentCode: Option<String>,
    dropConsentDescription: Option<String>,
    requirementsDescription: Option<String>,
}

pub async fn setup_nimbus() -> Result<(), Box<dyn Error>> {
    if Path::new("config.yaml").exists() {
        log::info!("Config file exists");
        let mut continue_prompt = QuerySelect::new(['Y', 'N'], |text, items| -> Vec<String> {
            text.parse::<usize>()
                .map(|query| {
                    items
                        .iter()
                        .filter(|num| query <= num.parse::<usize>().unwrap_or_default())
                        .map(|num| num.to_string())
                        .collect::<Vec<String>>()
                })
                .unwrap_or(items.clone())
        })
        .title("Config file exists. Continuing will overwrite the existing config file. Proceed?")
        .item_lines(2)
        .prompt()?;
        if continue_prompt.run()? == 'N'.to_string() {
            log::info!("Exiting...");
            std::process::exit(0);
        }
    }
    let config = parse_user_input().await?;
    write_config(config.clone()).expect("Failed to save config");
    log::info!("Saved config");

    Ok(())
}

async fn parse_user_input() -> Result<Config, Box<dyn Error>> {
    let mut courses_map: HashMap<String, Option<String>> = HashMap::new();
    let mut config = Config::default();
    let default_download_path_buf = download_dir().unwrap();
    let default_download_path = default_download_path_buf.to_str().unwrap();
    let mut download_path_prompt = Readline::default()
        .title(format!(
            "where is the directory for your downloads. leave blank for {}",
            &default_download_path,
        ))
        .validator(
            |text| Path::new(text).is_dir() || text.trim().is_empty(),
            |text| format!("Must be a valid directory. Got {} instead", text),
        )
        .prompt()?;

    let mut base_path_prompt = Readline::default()
        .title("where is your base directory for your files")
        .validator(
            |text| Path::new(text).is_dir(),
            |text| format!("Must be a valid directory. Got {} instead", text),
        )
        .prompt()?;

    let mut start_year_prompt = Readline::default()
        .title("When do you start university")
        .validator(
            |text| text.parse::<i32>().is_ok(),
            |text| format!("Must be a valid number. Got {} instead", text),
        )
        .prompt()?;

    let mut end_year_prompt = Readline::default()
        .title("When do you end university")
        .validator(
            |text| text.parse::<i32>().is_ok(),
            |text| format!("Must be a valid number. Got {} instead", text),
        )
        .prompt()?;

    let mut coop_prompt = Select::new([true, false])
        .title("Do you have co-op?")
        .lines(2)
        .prompt()?;
    let mut courses_prompt = Readline::default()
        .title("What courses are you taking this term? Please provide your answers in a comma separated list")
        .validator(
            |text| is_valid_course_list(text),
            |text| format!("Must be a comma seperated list. Got {} instead", text),
        )
        .prompt()?;

    let mut waterloo_api_key_prompt = Readline::default()
        .title("Enter your Waterloo OpenData API Key or leave blank to use richards")
        .prompt()?;

    let waterloo_api_key = waterloo_api_key_prompt.run()?;
    config.api_key = if waterloo_api_key.trim().is_empty() {
        std::env::var("WATERLOO_API_KEY")
            .expect("Missing Waterloo OpenData API Key")
            .to_string()
    } else {
        waterloo_api_key
    };

    courses_map = generate_course_map(config.clone().api_key).await?;
    let download_path_input = download_path_prompt.run()?;
    config.download_path = if download_path_input.trim().is_empty() {
        default_download_path_buf
    } else {
        PathBuf::from(download_path_input)
    };
    let mut term_prompt = Select::new(["1A", "1B", "2A", "2B", "3A", "3B", "4A", "4B"])
        .title("What term are you currently in?")
        .lines(4)
        .prompt()?;
    let current_term = term_prompt.run()?;
    match create_term_directories(&current_term, config.clone().base_path) {
        Ok(_) => log::info!("Created term directories"),
        Err(e) => log::error!("Failed to create term directories: {}", e),
    }
    config.base_path = PathBuf::from(base_path_prompt.run()?);
    config.start_year = start_year_prompt.run()?.parse().unwrap();
    config.end_year = end_year_prompt.run()?.parse().unwrap();
    config.coop = coop_prompt.run()?.parse().unwrap();
    config.courses = parse_course_list(courses_prompt.run()?, courses_map);
    config.current_term = current_term;
    Ok(config)
}

pub fn read_config() -> Result<Config, io::Error> {
    let contents = include_str!("../config.yaml");
    let parsed_data: Config = serde_yaml::from_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(parsed_data)
}
fn write_config(config: Config) -> Result<(), io::Error> {
    let yaml_string =
        serde_yaml::to_string(&config).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut file = File::create("config.yaml")?;
    file.write_all(yaml_string.as_bytes())?;
    Ok(())
}

fn create_term_directories(current_term: &str, base_dir: PathBuf) -> Result<(), io::Error> {
    let end_term_num = 4;
    let end_term_char = 'B';

    let curr_term_num = current_term
        .chars()
        .nth(0)
        .and_then(|c| c.to_digit(10))
        .ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid term number",
        ))?;
    let curr_term_char = current_term.chars().nth(1).ok_or(io::Error::new(
        io::ErrorKind::InvalidInput,
        "Invalid term character",
    ))?;

    for i in curr_term_num..=end_term_num {
        for j in curr_term_char as u8..=end_term_char as u8 {
            let term = format!("{}{}", i, j as char);
            let term_path = base_dir.join(term);
            if !term_path.exists() {
                fs::create_dir(term_path)?;
            }
        }
    }
    Ok(())
}

fn parse_course_list(s: String, courses_map: HashMap<String, Option<String>>) -> Vec<Course> {
    let mut courses: Vec<Course> = Vec::new();
    let course_list = csl_to_vec(s);
    for course in course_list {
        let course_name = course.clone().to_ascii_uppercase();
        let course_description = courses_map.get(&course).unwrap();
        match course_description {
            Some(course_description) => courses.push(Course {
                name: course_name,
                description: course_description.clone(),
            }),
            None => println!("{}: {}", course, "No description found"),
        }
    }
    courses
}

fn remove_whitespace(s: String) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn is_valid_course_list(s: &str) -> bool {
    s.contains(',') || (!s.contains(' ') && !s.contains(','))
}
fn csl_to_vec(s: String) -> Vec<String> {
    let str = remove_whitespace(s);
    str.split(',').map(|s| s.to_string()).collect()
}

fn generate_term_code() -> String {
    let current_date = Local::now();
    let year = current_date.year();
    let month = current_date.month();

    let a = if year < 2000 { "0" } else { "1" };
    let yy = format!("{:02}", year % 100);
    let term_month = match month {
        1..=4 => "1",        // January to April
        5..=8 => "5",        // May to August
        9..=12 => "9",       // September to December
        _ => unreachable!(), // This case should never happen
    };
    format!("{}{}{}", a, yy, term_month)
}

async fn generate_course_map(
    api_key: String,
) -> Result<HashMap<String, Option<String>>, Box<dyn Error>> {
    let url = "https://openapi.data.uwaterloo.ca/v3";
    let term_code = generate_term_code();
    let full_url = format!("{}/Courses/{}", url, term_code);
    let client = reqwest::Client::new();
    let mut courses_map: HashMap<String, Option<String>> = HashMap::new();
    log::info!("Grabbing course data...");
    log::info!("URL: {}", full_url);
    let response = client
        .get(full_url)
        .header("accept", "application/json")
        .header("x-api-key", api_key)
        .send()
        .await?;

    if response.status().is_success() {
        let courses: Vec<CourseInfo> = match response.json::<Vec<CourseInfo>>().await {
            Ok(courses) => courses,
            Err(e) => {
                eprintln!("Failed to deserialize response: {}", e);
                return Err(Box::new(e)); // Or handle the error as appropriate
            }
        };
        log::info!("Got course data");
        for course in courses {
            if let (Some(subject_code), Some(catalog_number)) =
                (&course.subjectCode, &course.catalogNumber)
            {
                let key = format!("{}{}", subject_code, catalog_number);
                courses_map.insert(key, course.description.clone());
            }
        }
    } else {
        eprintln!("Request failed with status: {}", response.status());
    }

    Ok(courses_map)
}
