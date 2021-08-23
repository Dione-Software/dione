fn main() -> Result<(), Box<dyn std::error::Error>> {
	tonic_build::compile_protos("../Server-Client-Protos/MessageStorage.proto")
		.expect("Got an error compiling protos");
	Ok(())
}
