//! Generate a fresh secp256k1 keypair for use as the agent wallet.
//! Prints the private key and address; you copy the key into .env.
//!
//! Usage: cargo run --bin gen-keypair

use alloy_signer_local::PrivateKeySigner;

fn main() {
    let signer = PrivateKeySigner::random();
    let key_bytes = signer.to_bytes();
    println!("== NEW AGENT KEYPAIR ==");
    println!("address:     {:#x}", signer.address());
    println!("private key: 0x{}", hex_encode(key_bytes.as_slice()));
    println!();
    println!("Add to .env:   HYPERCOPY_AGENT_KEY=0x{}", hex_encode(key_bytes.as_slice()));
    println!("Add to config: [wallet].agent_address = \"{:#x}\"", signer.address());
}

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}
