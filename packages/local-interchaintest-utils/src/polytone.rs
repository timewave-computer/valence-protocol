use sha2::{Digest, Sha512};

pub fn salt_for_proxy(
    connection_id: &str,
    counterparty_port: &str,
    remote_sender: &str,
) -> Vec<u8> {
    Sha512::default()
        .chain_update(connection_id.as_bytes())
        .chain_update(counterparty_port.as_bytes())
        .chain_update(remote_sender.as_bytes())
        .finalize()
        .to_vec()
}
