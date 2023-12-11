// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;

fn send_asset_from_asset_hub_rococo_to_asset_hub_westend(id: Location, amount: u128) {
	let signed_origin =
		<AssetHubRococo as Chain>::RuntimeOrigin::signed(AssetHubRococoSender::get().into());
	let asset_hub_westend_para_id = AssetHubWestend::para_id().into();
	let destination = Location::new(
		2,
		[GlobalConsensus(NetworkId::Westend), Parachain(asset_hub_westend_para_id)],
	);
	let beneficiary_id = AssetHubWestendReceiver::get();
	let beneficiary: Location =
		AccountId32Junction { network: None, id: beneficiary_id.into() }.into();
	let assets: Assets = (id, amount).into();
	let fee_asset_item = 0;

	// fund the AHR's SA on BHR for paying bridge transport fees
	let ahr_as_seen_by_bhr = BridgeHubRococo::sibling_location_of(AssetHubRococo::para_id());
	let sov_ahr_on_bhr = BridgeHubRococo::sovereign_account_id_of(ahr_as_seen_by_bhr);
	BridgeHubRococo::fund_accounts(vec![(sov_ahr_on_bhr.into(), 10_000_000_000_000u128)]);

	AssetHubRococo::execute_with(|| {
		assert_ok!(
			<AssetHubRococo as AssetHubRococoPallet>::PolkadotXcm::limited_reserve_transfer_assets(
				signed_origin,
				bx!(destination.into()),
				bx!(beneficiary.into()),
				bx!(assets.into()),
				fee_asset_item,
				WeightLimit::Unlimited,
			)
		);
	});

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubRococo,
			vec![
				// pay for bridge fees
				RuntimeEvent::Balances(pallet_balances::Event::Withdraw { .. }) => {},
				// message exported
				RuntimeEvent::BridgeWestendMessages(
					pallet_bridge_messages::Event::MessageAccepted { .. }
				) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});
	BridgeHubWestend::execute_with(|| {
		type RuntimeEvent = <BridgeHubWestend as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubWestend,
			vec![
				// message dispatched successfully
				RuntimeEvent::XcmpQueue(
					cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	});
}

#[test]
fn send_rocs_from_asset_hub_rococo_to_asset_hub_westend() {
	let roc_at_asset_hub_rococo: v3::Location = v3::Parent.into();
	let roc_at_asset_hub_westend =
		v3::Location::new(2, [v3::Junction::GlobalConsensus(v3::NetworkId::Rococo)]);
	let owner: AccountId = AssetHubWestend::account_id_of(ALICE);
	AssetHubWestend::force_create_foreign_asset(
		roc_at_asset_hub_westend,
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![],
	);
	let sov_ahw_on_ahr = AssetHubRococo::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Westend,
		AssetHubWestend::para_id(),
	);

	let rocs_in_reserve_on_ahr_before =
		<AssetHubRococo as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	let sender_rocs_before =
		<AssetHubRococo as Chain>::account_data_of(AssetHubRococoSender::get()).free;
	let receiver_rocs_before = AssetHubWestend::execute_with(|| {
		type Assets = <AssetHubWestend as AssetHubWestendPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			roc_at_asset_hub_westend,
			&AssetHubWestendReceiver::get(),
		)
	});

	let roc_at_asset_hub_rococo_latest: Location =
		roc_at_asset_hub_rococo.try_into().unwrap();
	let amount = ASSET_HUB_ROCOCO_ED * 1_000;
	send_asset_from_asset_hub_rococo_to_asset_hub_westend(roc_at_asset_hub_rococo_latest, amount);
	AssetHubWestend::execute_with(|| {
		type RuntimeEvent = <AssetHubWestend as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubWestend,
			vec![
				// issue ROCs on AHW
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == roc_at_asset_hub_rococo,
					owner: *owner == AssetHubWestendReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_rocs_after =
		<AssetHubRococo as Chain>::account_data_of(AssetHubRococoSender::get()).free;
	let receiver_rocs_after = AssetHubWestend::execute_with(|| {
		type Assets = <AssetHubWestend as AssetHubWestendPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			roc_at_asset_hub_westend,
			&AssetHubWestendReceiver::get(),
		)
	});
	let rocs_in_reserve_on_ahr_after =
		<AssetHubRococo as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;

	// Sender's balance is reduced
	assert!(sender_rocs_before > sender_rocs_after);
	// Receiver's balance is increased
	assert!(receiver_rocs_after > receiver_rocs_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(rocs_in_reserve_on_ahr_after, rocs_in_reserve_on_ahr_before + amount);
}

#[test]
fn send_wnds_from_asset_hub_rococo_to_asset_hub_westend() {
	let prefund_amount = 10_000_000_000_000u128;
	let wnd_at_asset_hub_rococo =
		v3::Location::new(2, [v3::Junction::GlobalConsensus(v3::NetworkId::Westend)]);
	let owner: AccountId = AssetHubWestend::account_id_of(ALICE);
	AssetHubRococo::force_create_foreign_asset(
		wnd_at_asset_hub_rococo,
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![(AssetHubRococoSender::get(), prefund_amount)],
	);

	// fund the AHR's SA on AHW with the WND tokens held in reserve
	let sov_ahr_on_ahw = AssetHubWestend::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Rococo,
		AssetHubRococo::para_id(),
	);
	AssetHubWestend::fund_accounts(vec![(sov_ahr_on_ahw.clone(), prefund_amount)]);

	let wnds_in_reserve_on_ahw_before =
		<AssetHubWestend as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	assert_eq!(wnds_in_reserve_on_ahw_before, prefund_amount);
	let sender_wnds_before = AssetHubRococo::execute_with(|| {
		type Assets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			wnd_at_asset_hub_rococo,
			&AssetHubRococoSender::get(),
		)
	});
	assert_eq!(sender_wnds_before, prefund_amount);
	let receiver_wnds_before =
		<AssetHubWestend as Chain>::account_data_of(AssetHubWestendReceiver::get()).free;

	let wnd_at_asset_hub_rococo_latest: Location =
		wnd_at_asset_hub_rococo.try_into().unwrap();
	let amount_to_send = ASSET_HUB_WESTEND_ED * 1_000;
	send_asset_from_asset_hub_rococo_to_asset_hub_westend(
		wnd_at_asset_hub_rococo_latest.clone(),
		amount_to_send,
	);
	AssetHubWestend::execute_with(|| {
		type RuntimeEvent = <AssetHubWestend as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubWestend,
			vec![
				// WND is withdrawn from AHR's SA on AHW
				RuntimeEvent::Balances(
					pallet_balances::Event::Withdraw { who, amount }
				) => {
					who: *who == sov_ahr_on_ahw,
					amount: *amount == amount_to_send,
				},
				// WNDs deposited to beneficiary
				RuntimeEvent::Balances(pallet_balances::Event::Deposit { who, .. }) => {
					who: *who == AssetHubWestendReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_wnds_after = AssetHubRococo::execute_with(|| {
		type Assets = <AssetHubRococo as AssetHubRococoPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(wnd_at_asset_hub_rococo, &AssetHubRococoSender::get())
	});
	let receiver_wnds_after =
		<AssetHubWestend as Chain>::account_data_of(AssetHubWestendReceiver::get()).free;
	let wnds_in_reserve_on_ahw_after =
		<AssetHubWestend as Chain>::account_data_of(sov_ahr_on_ahw).free;

	// Sender's balance is reduced
	assert!(sender_wnds_before > sender_wnds_after);
	// Receiver's balance is increased
	assert!(receiver_wnds_after > receiver_wnds_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(wnds_in_reserve_on_ahw_after, wnds_in_reserve_on_ahw_before - amount_to_send);
}
