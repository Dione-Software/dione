fn main() -> Result<(), Box<dyn std::error::Error>> {
	tonic_build::configure()
		.type_attribute("ServerAddressType", "#[derive(serde::Serialize, serde::Deserialize)]")
		.compile(&["../Server-Client-Protos/MessageStorage.proto"], &["../Server-Client-Protos/"])?;
	Ok(())
}
