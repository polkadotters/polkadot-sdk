// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

use polkadot_subsystem_bench::{
	availability::{
		benchmark_availability_read, prepare_test, DataAvailabilityReadOptions,
		TestDataAvailability, TestState,
	},
	configuration::{PeerLatency, TestConfiguration},
};

fn main() -> Result<(), String> {
	let mut messages = vec![];
	let options = DataAvailabilityReadOptions { fetch_from_backers: true };
	let mut config = TestConfiguration::default();
	config.latency = Some(PeerLatency { mean_latency_ms: 100, std_dev: 1.0 });
	config.n_validators = 300;
	config.n_cores = 20;
	config.min_pov_size = 5120;
	config.max_pov_size = 5120;
	config.peer_bandwidth = 52428800;
	config.bandwidth = 52428800;
	config.num_blocks = 3;
	config.connectivity = 90;
	config.generate_pov_sizes();

	let mut state = TestState::new(&config);
	let (mut env, _protocol_config) =
		prepare_test(config, &mut state, TestDataAvailability::Read(options), false);
	let usage = env.runtime().block_on(benchmark_availability_read(
		"data_availability_read",
		&mut env,
		state,
	));

	println!("{}", usage);

	messages.extend(usage.check_network_usage(&[
		("Received from peers", 102400.000, 102500.000),
		("Sent to peers", 0.000, 100.000),
	]));
	messages.extend(usage.check_cpu_usage(&[("availability-recovery", 4.200, 4.300)]));

	if messages.is_empty() {
		Ok(())
	} else {
		eprintln!("{}", messages.join("\n"));
		Err("Regressions found".to_string())
	}
}
