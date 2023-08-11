use std::net::IpAddr;

use serde::Serialize;

use crate::dns::{Config, Resolver};

use super::args::LookupArgs;

pub fn run(args: &LookupArgs) -> anyhow::Result<()> {
    let config = Config::new().with_suggested_servers();
    let resolver = Resolver::new(config)?;

    let lookup = resolver.lookup_ip_address(&args.name)?;

    if args.json {
        let doc = OutputDoc {
            ip_addresses: lookup.ip_addresses().to_vec(),
            text_record: lookup.to_record_string()
        };
        let output = serde_json::to_string_pretty(&doc)?;
        println!("{}", output);
    } else {
        for address in lookup.ip_addresses() {
            println!("{}", address);
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct OutputDoc {
    ip_addresses: Vec<IpAddr>,
    text_record: String,
}
