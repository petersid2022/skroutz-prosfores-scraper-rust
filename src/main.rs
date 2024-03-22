use anyhow::Error;
use clap::Parser;
use rand::seq::SliceRandom;
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Instant;
use std::usize;
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Set the number of products to print
    #[arg(short, long = "number-of-items", default_value_t = 5)]
    n: usize,

    /// Set the number of pages to scrape
    #[arg(short, long = "number-of-pages", default_value_t = 5)]
    p: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let t1 = Instant::now();

    let client = reqwest::Client::new();
    let data = scrape(&client, args.n, args.p).await;
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

fn calculate_discount(original_price: &str, discounted_price: &str) -> Result<f32, Error> {
    let original_price_str: String = original_price
        .chars()
        .filter(|c| char::is_numeric(*c) || *c == ',')
        .collect();
    let discounted_price_str: String = discounted_price
        .chars()
        .filter(|c| char::is_numeric(*c) || *c == ',')
        .collect();

    let original_price_int: f32 = original_price_str.replace(',', ".").parse()?;
    let discounted_price_int: f32 = discounted_price_str.replace(',', ".").parse()?;

    let discount = 100.0 * (original_price_int - discounted_price_int) / original_price_int;

    Ok(discount)
}

async fn scrape(
    client: &Client,
    number_of_items: usize,
    number_of_pages: usize,
) -> Result<Vec<Function>, Error> {
    let base_url = "https://www.skroutz.gr/deals?order_by=".to_string();
    let url = format!("{}recommended&recent=1&page={}", base_url, number_of_pages);

    let response = client.get(url).send().await?;

    let body = response.text().await?;
    let fragment = Html::parse_fragment(&body);
    let selector = Selector::parse(r#"li[class="cf card"]"#).unwrap();
    let mut data: Vec<Function> = Vec::new();

    for card in fragment.select(&selector) {
        let card_body = Html::parse_fragment(&card.html());
        let selector = Selector::parse(r#"div[class="card-content"]"#).unwrap();
        let products = card_body.select(&selector);
        for product in products {
            let product_body = Html::parse_fragment(&product.html());

            let strike_selector = Selector::parse("strike").unwrap();
            let a_selector = Selector::parse(r#"a[class="js-sku-link sku-link"]"#).unwrap();

            let original_price = product_body
                .select(&strike_selector)
                .next()
                .map(|del| del.text().collect::<String>());

            let discounted_price = product_body
                .select(&a_selector)
                .next()
                .map(|a| a.text().collect::<String>());

            let discounted_price = discounted_price
                .unwrap()
                .chars()
                .skip_while(|c| !char::is_numeric(*c))
                .collect::<String>();

            let title = product_body
                .select(&a_selector)
                .next()
                .map(|a| a.value().attr("title").unwrap_or_default());

            let link = product_body
                .select(&a_selector)
                .next()
                .map(|a| a.value().attr("href").unwrap_or_default());

            if let (Some(title), Some(original_price), discounted_price, Some(link)) =
                (title, original_price, discounted_price, link)
            {
                let url = format!("https://skroutz.gr{}", link);
                let link = create_link(url, "link".to_string());
                let discount = calculate_discount(&original_price, &discounted_price)?;
                data.push(Function::new(
                    title.trim(),
                    original_price.trim(),
                    discounted_price.trim(),
                    &discount.to_string(),
                    link.trim(),
                ));
            }
        }
    }

    // Shuffle the data
    data.truncate(number_of_items);

    let mut rng = rand::thread_rng();
    data.shuffle(&mut rng);

    Ok(data)
}
