use structopt::StructOpt;
use std::path::PathBuf;
use dione_net_lib::Client;
use dialoguer::{Input, Select, Confirm, Editor};
use dialoguer::theme::ColorfulTheme;
use uuid::Uuid;
use std::ops::Not;

#[derive(Debug, StructOpt)]
struct Opt {
	#[structopt(long, default_value = "http://127.0.0.1:8000")]
	server: String,
	#[structopt(parse(from_os_str))]
	db_path: PathBuf,
	#[structopt(long, default_value = "3")]
	share_number: usize
}

fn main() -> anyhow::Result<()> {
	let opt = Opt::from_args();
	let mut client = Client::new(opt.db_path, opt.share_number)?;
	client.connect(opt.server)?;

	let host_uuid = client.get_uuid();
	println!("This is your Uuid => {}", host_uuid);

	client.provide_bundle().expect("Error providing bundle");

	loop {
		let possibilities = vec!["Receive Message", "Send Message", "Add User", "Exit"];
		let selection = Select::with_theme(&ColorfulTheme::default())
			.items(&possibilities)
			.with_prompt("What do you want to do?")
			.default(0)
			.interact()
			.unwrap();
		match selection {
			0 => {
				let possible_users = client.get_peers();
				let selected_user = Select::with_theme(&ColorfulTheme::default())
					.items(&possible_users)
					.with_prompt("From which user do you want to receive a message?")
					.default(0)
					.interact().unwrap();
				let user = possible_users.get(selected_user).unwrap().to_owned();
				let received_message_bytes = client.recv_message(user).expect("Error receiving message");
				let received_message = String::from_utf8(received_message_bytes).expect("Error parsing back to string");
				println!("Message from {}:\n{}", user, received_message);
			}
			1 => {
				let possible_users = client.get_peers();
				let selected_user = Select::with_theme(&ColorfulTheme::default())
					.items(&possible_users)
					.with_prompt("To what user do you want to send a message?")
					.default(0)
					.interact().unwrap();
				let user = possible_users.get(selected_user).unwrap().to_owned();
				let message: String = Editor::new()
					.edit("Enter the message you want to send.")
					.unwrap()
					.unwrap();
				let message_bytes = message.as_bytes();
				client.send_message(user, message_bytes).expect("Error sending message");
				println!("Success! Send message.");
			}
			2 => {
				let peer_uuid_string: String = Input::with_theme(&ColorfulTheme::default())
					.with_prompt("Enter remote user Uuid")
					.interact_text()
					.unwrap();
				let peer_uuid = Uuid::parse_str(&peer_uuid_string).expect("Faulty uuid provided");
				let add_user_possibilities = vec!["Initiate contact", "React on invitation"];

				let add_user_selection = Select::with_theme(&ColorfulTheme::default())
					.items(&add_user_possibilities)
					.with_prompt("What do you want to do?")
					.default(0)
					.interact()
					.unwrap();

				match add_user_selection {
					0 => {
						if Confirm::with_theme(&ColorfulTheme::default())
							.with_prompt("Is remote peer connected?")
							.interact()
							.unwrap().not()
						{
							println!("Looks like you don't want to continue");
							continue
						}
						client.init_one_session(peer_uuid).expect("Error performing stage one of establishing connection");
						println!("Peer has to continue on \"React on invitation\"");
						if Confirm::with_theme(&ColorfulTheme::default())
							.with_prompt("Did remote peer continue as told?")
							.interact()
							.unwrap().not()
						{
							println!("Looks like you didn't do as told");
							continue
						}
						client.init_three_session(peer_uuid).expect("Error starting session");
						println!("Congratulations you just added a new user with the Uuid => {}", peer_uuid);
						println!("Peer has to continue on there side");
					},
					1 => {
						if Confirm::with_theme(&ColorfulTheme::default())
							.with_prompt("Did remote peer start contact process?")
							.interact()
							.unwrap()
							.not()
						{
							println!("Looks like you don't want to continue");
							continue
						}
						client.init_two_session(peer_uuid).unwrap();
						if Confirm::with_theme(&ColorfulTheme::default())
							.with_prompt("Did remote peer continued the process?")
							.interact()
							.unwrap()
							.not()
						{
							println!("Looks like you don't want to continue. Aborting");
							continue
						}
						client.start_session(peer_uuid).unwrap();
						println!("Congratulations you just added a new user with the Uuid => {}", peer_uuid);
					}
					_ => {
						println!("This is a mistake");
					}
				}


			}
			3 => {
				println!("Exiting");
				break
			}
			_ => {
				println!("This is a mistake");
			}
		}
	}
	Ok(())
}