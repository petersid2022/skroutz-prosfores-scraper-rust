use anyhow::Error;
use rand::seq::SliceRandom;
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Instant;
use std::{env, process::exit, usize};
use tabled::{
    settings::{
        object::Columns, style::Style, themes::ColumnNames, Alignment, Color, Modify, Width,
    },
    Table, Tabled,
};

#[derive(Debug, Tabled)]
struct Function {
    #[tabled(rename = " Name ")]
    name: String,
    #[tabled(rename = " Old price ")]
    old_price: String,
    #[tabled(rename = " New price ")]
    new_price: String,
    #[tabled(rename = " % Discount ")]
    discount: String,
    #[tabled(rename = " Link ")]
    link: String,
}

impl Function {
    fn new(decl: &str, name: &str, ret_type: &str, disc: &str, link: &str) -> Self {
        Self {
            name: decl.to_string(),
            old_price: name.to_string(),
            new_price: ret_type.to_string(),
            discount: disc.to_string(),
            link: link.to_string(),
        }
    }
}

fn help() {
    println!(
        "Usage:
        -n [number]:    Set the number of products to print (default: 5)
        -p [number]:    Set the number of pages to scrape (default: 1)"
    );
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let t1 = Instant::now();
    let args: Vec<String> = env::args().collect();
    let mut number_of_items: usize = 5;
    let mut number_of_pages: usize = 1;

    if args.len() > 1 && args.len() != 3 {
        help();
        exit(0);
    }

    if args.len() == 3 {
        match args[1].as_str() {
            "-n" => match args[2].parse::<u32>() {
                Ok(num) => {
                    if num == 0 {
                        help();
                        exit(0);
                    }
                    number_of_items = num as usize;
                }
                _ => {
                    println!("Invalid argument.");
                    help();
                    exit(0);
                }
            },
            "-p" => match args[2].parse::<u32>() {
                Ok(f) => {
                    if f == 0 {
                        help();
                        exit(0);
                    }
                    number_of_pages = f as usize;
                }
                _ => {
                    println!("Invalid argument.");
                    help();
                    exit(0);
                }
            },
            _ => {
                help();
                exit(0);
            }
        }
    }

    let client = reqwest::Client::new();
    let data = scrape(&client, number_of_items, number_of_pages).await;
    let mut table = match data {
        Ok(data) => Table::new(data),
        Err(err) => {
            eprintln!("error: {}", err);
            return;
        }
    };

    table
        .with(Style::modern())
        .with(Modify::new(Columns::first()).with(Width::wrap(50).keep_words()))
        .with(
            ColumnNames::default()
                .color(Color::BOLD | Color::BG_GREEN | Color::FG_BLACK)
                .alignment(Alignment::center()),
        );

    let elapsed_duration = t1.elapsed();

    println!("{table}");
    println!("(c) Peter Sideris 2024");
    println!("Elapsed time: {:?}", elapsed_duration);
}

fn create_link(x: String, y: String) -> String {
    let out = format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", x, y);
    out
}

fn calculate_discount(original_price: &String, discounted_price: &String) -> Result<f32, Error> {
    let original_price_str: String = original_price
        .chars()
        .filter(|c| char::is_numeric(*c) || *c == ',')
        .collect();
    let discounted_price_str: String = discounted_price
        .chars()
        .filter(|c| char::is_numeric(*c) || *c == ',')
        .collect();

    let original_price_int: f32 = original_price_str.replace(",", ".").parse()?;
    let discounted_price_int: f32 = discounted_price_str.replace(",", ".").parse()?;

    let discount = 100.0 * (original_price_int - discounted_price_int) / original_price_int;

    Ok(discount)
}

async fn scrape(
    client: &Client,
    number_of_items: usize,
    number_of_pages: usize,
) -> Result<Vec<Function>, Error> {
    let base_url = "https://www.skroutz.gr/price-drops?order_by=".to_string();
    let url = format!("{}recommended&recent=1&page={}", base_url, number_of_pages);

    let response = client.get(url).send().await?;

    let body = response.text().await?;
    let fragment = Html::parse_fragment(&body);
    let selector = Selector::parse(".sku-card.js-sku").unwrap();
    let mut data: Vec<Function> = Vec::new();

    for card in fragment.select(&selector) {
        let card_body = Html::parse_fragment(&card.html());
        let selector = Selector::parse(".sku-card-info").unwrap();
        let products = card_body.select(&selector);
        for product in products {
            let product_body = Html::parse_fragment(&product.html());

            let del_selector = Selector::parse("del").unwrap();
            let a_selector = Selector::parse("p.sku-card-price a").unwrap();

            let original_price = product_body
                .select(&del_selector)
                .next()
                .map(|del| del.text().collect::<String>());

            let discounted_price = product_body
                .select(&a_selector)
                .next()
                .map(|a| a.text().collect::<String>());

            let title = product_body
                .select(&a_selector)
                .next()
                .map(|a| a.value().attr("title").unwrap_or_default());

            let link = product_body
                .select(&a_selector)
                .next()
                .map(|a| a.value().attr("href").unwrap_or_default());

            if let (Some(title), Some(original_price), Some(discounted_price), Some(link)) =
                (title, original_price, discounted_price, link)
            {
                let url = format!("https://skroutz.gr{}", link);
                let link = create_link(url, "link".to_string());
                let discount = calculate_discount(&original_price, &discounted_price)?;
                data.push(Function::new(
                    &title.trim().to_string(),
                    &original_price.trim().to_string(),
                    &discounted_price.trim().to_string(),
                    &discount.to_string(),
                    &link.trim().to_string(),
                ));
            }
        }
    }

    data = data.drain(0..number_of_items).collect();

    let mut rng = rand::thread_rng();
    data.shuffle(&mut rng);

    Ok(data)
}
