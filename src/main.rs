use std::{borrow::Cow, convert::Infallible};

use warp::{hyper::Response, Filter};

mod db;

#[derive(Debug, serde::Deserialize)]
struct Questname {
	quest: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Quest {
	pub id: String,
	pub artist: String,
	pub poem: String,
	pub next: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Image {
	pub id: i64,
	pub quest_id: String,
	pub data: Vec<u8>,
}

#[derive(Debug, serde::Serialize)]
struct Message {
	success: bool,
	message: Cow<'static, str>,
}

#[derive(Debug, serde::Deserialize)]
struct CreateQuest {
	quest: Quest,
	images: Vec<Vec<u8>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let pool = db::Pool::connect("sqlite:./db.sqlite3").await?;

	macro_rules! with_data {
		($data: ident) => {
			warp::any().map({
				let data = $data.clone();
				move || data.clone()
			})
		};
	}

	let quest = warp::path!("quest")
		.and(warp::get())
		.and(warp::query())
		.and(with_data!(pool))
		.and_then(|q: Questname, pool| async move {
			let gal = sqlx::query_as!(Quest, "SELECT * FROM quests WHERE id = $1", q.quest)
				.fetch_optional(&pool)
				.await
				.ok()
				.flatten();

			match gal {
				Some(gal) => Ok::<_, Infallible>(warp::reply::json(
					&serde_json::to_value(&gal).expect("Error jsoning"),
				)),
				None => Ok(warp::reply::json(
					&serde_json::to_value(&Message {
						success: false,
						message: Cow::Borrowed("quest not found"),
					})
					.expect("json"),
				)),
			}
		});

	let images = warp::path!("quest" / "images")
		.and(warp::get())
		.and(warp::query())
		.and(with_data!(pool))
		.and_then(|q: Questname, pool| async move {
			let images = sqlx::query_as!(
				Image,
				"SELECT * FROM images WHERE quest_id = $1 ORDER BY id ASC",
				q.quest
			)
			.fetch_all(&pool)
			.await
			.expect("DB error");
			Ok::<_, Infallible>(warp::reply::json(
				&serde_json::to_value(&images).expect("json error"),
			))
		});

	let read_routes = quest.or(images);

	let create_quest = warp::path!("quest")
		.and(warp::post())
		.and(warp::body::json())
		.and(with_data!(pool))
		.and_then(|data: CreateQuest, pool| async move {
			if sqlx::query!("SELECT id FROM quests WHERE id = $1", data.quest.id)
				.fetch_optional(&pool)
				.await
				.expect("db error")
				.is_some()
			{
				return Ok::<_, Infallible>(warp::reply::json(
					&serde_json::to_value(&Message {
						success: false,
						message: Cow::Borrowed("quest already exists"),
					})
					.expect("json error"),
				));
			}

			sqlx::query!(
				"INSERT INTO quests (id, artist, poem, next) VALUES ($1, $2, $3, $4)",
				data.quest.id,
				data.quest.artist,
				data.quest.poem,
				data.quest.next
			)
			.execute(&pool)
			.await
			.expect("db error");

			for image in data.images {
				sqlx::query!(
					"INSERT INTO images (quest_id, data) VALUES ($1, $2)",
					data.quest.id,
					image
				)
				.execute(&pool)
				.await
				.expect("db error");
			}

			Ok(warp::reply::json(
				&serde_json::to_value(&Message {
					success: true,
					message: Cow::Borrowed("created successfully"),
				})
				.expect("json error"),
			))
		});

	let server = warp::serve(read_routes.or(create_quest));

	server.run(([0, 0, 0, 0], 4444)).await;

	Ok(())
}
