#[allow(unused_imports)]
use anyhow::{Context, Error};
use reqwest::Client;
use scraper::{Html, Selector};
use tabled::{
    settings::{
        object::Columns, style::Style, themes::ColumnNames, Alignment, Color, Modify, Width,
    },
    Table, Tabled,
};

#[derive(Debug, Tabled)]
struct Function {
    name: String,
    old_price: String,
    new_price: String,
}

impl Function {
    fn new(decl: &str, name: &str, ret_type: &str) -> Self {
        Self {
            name: decl.to_string(),
            old_price: name.to_string(),
            new_price: ret_type.to_string(),
        }
    }
}

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();
    let data = scrape(&client).await;
    let mut table = Table::new(data.unwrap());

    table
        .with(Style::modern())
        .with(
            ColumnNames::default()
                .color(Color::BOLD | Color::BG_GREEN | Color::FG_BLACK)
                .alignment(Alignment::center()),
        )
        .with(Modify::new(Columns::first()).with(Width::wrap(50).keep_words()));

    println!("{table}");
}

async fn scrape(client: &Client) -> Result<Vec<Function>, Error> {
    let response = client
        .get("https://www.skroutz.gr/price-drops")
        .send()
        .await?;

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

            if let (Some(title), Some(original_price), Some(discounted_price)) =
                (title, original_price, discounted_price)
            {
                data.push(Function::new(
                    &title.trim().to_string(),
                    &original_price.trim().to_string(),
                    &discounted_price.trim().to_string(),
                ));
            }
        }
    }

    Ok(data)
}
