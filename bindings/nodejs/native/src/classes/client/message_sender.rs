// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_client::{
    api::PreparedTransactionData,
    bee_message::prelude::{Address, MessageId, Payload, UtxoInput, UnlockBlock, ReferenceUnlock, SignatureUnlock, Ed25519Signature, UnlockBlocks, TransactionPayloadBuilder, Essence, RegularEssence},
    Seed,
};
use neon::prelude::*;

use super::{parse_address, Api, ClientTask};

use std::{ops::Range, str::FromStr, collections::{HashMap}};

pub struct MessageSender {
    client_id: String,
    index: Option<Vec<u8>>,
    data: Option<Vec<u8>>,
    parents: Option<Vec<MessageId>>,
    seed: Option<String>,
    account_index: Option<usize>,
    initial_address_index: Option<usize>,
    inputs: Vec<UtxoInput>,
    input_range: Option<Range<usize>>,
    outputs: Vec<(Address, u64)>,
    dust_allowance_outputs: Vec<(Address, u64)>,
}

declare_types! {
    pub class JsMessageSender for MessageSender {
        init(mut cx) {
            let client_id = cx.argument::<JsString>(0)?.value();
            Ok(MessageSender {
                client_id,
                index: None,
                data: None,
                parents: None,
                seed: None,
                account_index: None,
                initial_address_index:None,
                inputs: Vec::new(),
                input_range: None,
                outputs: Vec::new(),
                dust_allowance_outputs: Vec::new(),
            })
        }

        method seed(mut cx) {
            let seed = cx.argument::<JsString>(0)?.value();

            // validate the seed
            Seed::from_bytes(&hex::decode(&seed).expect("invalid seed hex"));

            {
                let mut this = cx.this();
                let guard = cx.lock();
                let send_seed = &mut this.borrow_mut(&guard).seed;
                send_seed.replace(seed);
            }

            Ok(cx.this().upcast())
        }

        method index(mut cx) {
            let mut index: Vec<u8> = vec![];
            let index_js_array = cx.argument::<JsArray>(0)?;
            let js_index: Vec<Handle<JsValue>> = index_js_array.to_vec(&mut cx)?;
            for value in js_index {
                let value: Handle<JsNumber> = value.downcast_or_throw(&mut cx)?;
                index.push(value.value() as u8);
            }

            {
                let mut this = cx.this();
                let guard = cx.lock();
                let index_ = &mut this.borrow_mut(&guard).index;
                index_.replace(index);
            }

            Ok(cx.this().upcast())
        }

        method data(mut cx) {
            let mut data: Vec<u8> = vec![];
            let data_js_array = cx.argument::<JsArray>(0)?;
            let js_data: Vec<Handle<JsValue>> = data_js_array.to_vec(&mut cx)?;
            for value in js_data {
                let value: Handle<JsNumber> = value.downcast_or_throw(&mut cx)?;
                data.push(value.value() as u8);
            }

            {
                let mut this = cx.this();
                let guard = cx.lock();
                let data_ = &mut this.borrow_mut(&guard).data;
                data_.replace(data);
            }

            Ok(cx.this().upcast())
        }

        method parents(mut cx) {
            let mut data: Vec<MessageId> = vec![];
            let data_js_array = cx.argument::<JsArray>(0)?;
            let js_data: Vec<Handle<JsValue>> = data_js_array.to_vec(&mut cx)?;
            for parent in js_data {
                let value: Handle<JsString> = parent.downcast_or_throw(&mut cx)?;
                let parent = MessageId::from_str(&value.value()).expect("invalid parent message id");
                data.push(parent);
            }
            {
                let mut this = cx.this();
                let guard = cx.lock();
                let send_parents = &mut this.borrow_mut(&guard).parents;
                send_parents.replace(data);
            }

            Ok(cx.this().upcast())
        }

        method accountIndex(mut cx) {
            let account_index = cx.argument::<JsNumber>(0)?.value() as usize;
            {
                let mut this = cx.this();
                let guard = cx.lock();
                let send_account_index = &mut this.borrow_mut(&guard).account_index;
                send_account_index.replace(account_index);
            }

            Ok(cx.this().upcast())
        }

        method initialAddressIndex(mut cx) {
            let index = cx.argument::<JsNumber>(0)?.value() as usize;
            {
                let mut this = cx.this();
                let guard = cx.lock();
                let send_address_index = &mut this.borrow_mut(&guard).initial_address_index;
                send_address_index.replace(index);
            }

            Ok(cx.this().upcast())
        }

        method output(mut cx) {
            let address = cx.argument::<JsString>(0)?.value();
            let address = parse_address(address.as_str()).expect("invalid address");
            let value = cx.argument::<JsNumber>(1)?.value() as u64;
            {
                let mut this = cx.this();
                let guard = cx.lock();
                let outputs = &mut this.borrow_mut(&guard).outputs;
                outputs.push((address, value));
            }

            Ok(cx.this().upcast())
        }

        method dustAllowanceOutput(mut cx) {
            let address = cx.argument::<JsString>(0)?.value();
            let address = parse_address(address.as_str()).expect("invalid address");
            let value = cx.argument::<JsNumber>(1)?.value() as u64;
            {
                let mut this = cx.this();
                let guard = cx.lock();
                let dust_allowance_outputs = &mut this.borrow_mut(&guard).dust_allowance_outputs;
                dust_allowance_outputs.push((address, value));
            }

            Ok(cx.this().upcast())
        }

        method input(mut cx) {
            let output_id = cx.argument::<JsString>(0)?.value();
            let utxo_input = UtxoInput::from_str(&output_id).expect("invalid UTXO input");
            {
                let mut this = cx.this();
                let guard = cx.lock();
                let inputs = &mut this.borrow_mut(&guard).inputs;
                inputs.push(utxo_input);
            }

            Ok(cx.this().upcast())
        }

        method inputRange(mut cx){
            let start = cx.argument::<JsNumber>(0)?.value() as usize;
            let end = cx.argument::<JsNumber>(1)?.value() as usize;
            {
                let mut this = cx.this();
                let guard = cx.lock();
                let input_range = &mut this.borrow_mut(&guard).input_range;
                input_range.replace(start..end);
            }
            Ok(cx.this().upcast())
        }

        method prepareTransaction(mut cx){
            let cb = cx.argument::<JsFunction>(0)?;
            {
                let this = cx.this();
                let guard = cx.lock();
                let ref_ = &(*this.borrow(&guard));
                let client_task = ClientTask {
                    client_id: ref_.client_id.clone(),
                    api: Api::PrepareTransaction {
                        seed: ref_.seed.as_ref().map(|seed| Seed::from_bytes(&hex::decode(&seed).expect("invalid seed hex"))),
                        index: ref_.index.clone(),
                        data: ref_.data.clone(),
                        parents: ref_.parents.clone(),
                        account_index: ref_.account_index,
                        initial_address_index: ref_.initial_address_index,
                        inputs: ref_.inputs.clone(),
                        input_range: ref_.input_range.clone(),
                        outputs: ref_.outputs.clone(),
                        dust_allowance_outputs: ref_.dust_allowance_outputs.clone(),
                    },
                };
                client_task.schedule(cb);
            }

            Ok(cx.undefined().upcast())
        }

        method signTransaction(mut cx){
            let transaction_data_string = cx.argument::<JsString>(0)?.value();
            let transaction_data: PreparedTransactionData = serde_json::from_str(&transaction_data_string).expect("invalid prepared transaction data");
            let seed = cx.argument::<JsString>(1)?.value();
            let seed = Seed::from_bytes(&hex::decode(&seed).expect("invalid seed hex"));
            let inputs_range  = if cx.len() > 4 {
                let start: Option<usize> = match cx.argument_opt(2) {
                    Some(arg) => {
                        Some(arg.downcast::<JsNumber>().or_throw(&mut cx)?.value() as usize)
                    },
                    None => None,
                };
                let end: Option<usize> = match cx.argument_opt(3) {
                    Some(arg) => {
                        Some(arg.downcast::<JsNumber>().or_throw(&mut cx)?.value() as usize)
                    },
                    None => None,
                };
                if start.is_some() && end.is_some() {
                    //save to unwrap since we checked if they are some
                    Some(start.expect("no start index")..end.expect("no end index"))
                }else{None}
            }else{None};

            let cb = cx.argument::<JsFunction>(cx.len()-1)?;
            {
                let this = cx.this();
                let guard = cx.lock();
                let ref_ = &(*this.borrow(&guard));
                let client_task = ClientTask {
                    client_id: ref_.client_id.clone(),
                    api: Api::SignTransaction {
                        transaction_data,
                        seed,
                        inputs_range,
                    },
                };
                client_task.schedule(cb);
            }

            Ok(cx.undefined().upcast())
        }

        method externalSignTransaction(mut cx){
            let essence_string = cx.argument::<JsString>(0)?.value();
            let essence: Essence = serde_json::from_str(&essence_string).expect("invalid transaction data(essence)");

            //let mut address_index_recorders = transaction_data.address_index_recorders;
            let hashed_essence = essence.hash();
            //let mut signature_indexes = HashMap::<String, usize>::new();
            //address_index_recorders.sort_by(|a, b| a.input.cmp(&b.input));

            // Check if current path is same as previous path
            // If so, add a reference unlock block
            // Format to differentiate between public and internal addresses
            //let index = format!("{}{}", recorder.address_index, recorder.internal);
            //if let Some(block_index) = signature_indexes.get(&index) {
                //unlock_blocks.push(UnlockBlock::Reference(ReferenceUnlock::new(*block_index as u16).unwrap()));
            //} else {
                // If not, we need to create a signature unlock block
                let external_signer: Handle<JsObject> = cx.argument::<JsObject>(1)?;

                let jsvalue_sign: Handle<JsValue> = external_signer.get(&mut cx, "sign").unwrap();
                let jsfn_sign: Handle<JsFunction> = jsvalue_sign.downcast_or_throw::<JsFunction, _>(&mut cx)?;
                
                let uint8_ctor = cx.global()
                    .get(&mut cx, "Uint8Array")?
                    .downcast_or_throw::<JsFunction, _>(&mut cx)?;
                let mut buf = cx.array_buffer(hashed_essence.len().try_into().unwrap())?;
                cx.borrow_mut(&mut buf, |buf| {
                    buf.as_mut_slice::<u8>().copy_from_slice(&hashed_essence);
                });
                let jsobject_hashedessence: Handle<JsObject> = uint8_ctor.construct(&mut cx, [buf])?;

                let jsbuffer_signature: Handle<JsBuffer> = jsfn_sign.call(&mut cx, external_signer, vec!(jsobject_hashedessence))?
                    .downcast_or_throw::<JsBuffer, _>(&mut cx)?;

                let jsbuffer_signature: Handle<JsBuffer> = jsbuffer_signature.downcast_or_throw::<JsBuffer, _>(&mut cx)?;
                let slice_signature: &[u8] = cx.borrow(&jsbuffer_signature, |data: neon::borrow::Ref<neon::types::BinaryData>| {
                    data.as_slice::<u8>()
                });
                let signature: [u8; 64] = slice_signature.try_into().unwrap();

                let jsvalue_publickey: Handle<JsValue> = external_signer.get(&mut cx, "getPublicKey").unwrap();
                let jsfn_publickey: Handle<JsFunction> = jsvalue_publickey.downcast_or_throw::<JsFunction, _>(&mut cx)?;
                let jsvalue_publickey: Handle<JsValue> = jsfn_publickey.call::<CallContext<_>, JsObject, JsObject, Vec<Handle<JsObject>>>(&mut cx, external_signer, vec![])?;
                let jsbuffer_publickey: Handle<JsBuffer> = jsvalue_publickey.downcast_or_throw::<JsBuffer, _>(&mut cx)?;
                let slice_publickey: &[u8] = cx.borrow(&jsbuffer_publickey, |data: neon::borrow::Ref<neon::types::BinaryData>| {
                    data.as_slice::<u8>()
                });
                let public_key: [u8; 32] = slice_publickey.try_into().unwrap();

                // The signature unlock block needs to sign the hash of the entire transaction essence of the
                // transaction payload
                let signature: Box<[u8; 64]> = Box::new(signature);
                //signature_indexes.insert(index, current_block_index);
            //}
    
            let unlock_block = UnlockBlock::Signature(SignatureUnlock::Ed25519(Ed25519Signature::new(
                public_key, *signature,
            )));
            
            let cb = cx.argument::<JsFunction>(cx.len()-1)?;
            {
                let this = cx.this();
                let guard = cx.lock();
                let ref_ = &(*this.borrow(&guard));
                let client_task = ClientTask {
                    client_id: ref_.client_id.clone(),
                    api: Api::FinishExternalSignTransaction {
                        unlock_block,
                    },
                };
                client_task.schedule(cb);
            }            

            Ok(cx.undefined().upcast())

            //Ok(Payload::Transaction(Box::new(payload)))
            /*

            let object: Handle<JsObject> = cx.argument::<JsObject>(0)?;
            let sign: Handle<JsValue> = object.get(&mut cx, "sign").unwrap();
            let function: Handle<JsFunction> = sign.downcast_or_throw::<JsFunction, _>(&mut cx)?;
            //let message: Handle<JsArray> = cx.empty_array();
            let message = cx.buffer(3)?;
            let n = cx.number(3 as u8);
            message.set(&mut cx, 0, n)?;
            message.set(&mut cx, 1, n)?;
            message.set(&mut cx, 2, n)?;
        
            let signResult: Handle<JsValue> = function.call(&mut cx, object, vec![message])?;
            let array: Handle<JsArray> = signResult.downcast_or_throw::<JsArray, _>(&mut cx)?;
            let vec: Vec<Handle<JsValue>> = array.to_vec(&mut cx)?;
            let slice: &mut [u8; 32] = &mut [0 as u8;32];
            for (i, jsValue) in vec.iter().enumerate() {
                let jsByte: Handle<JsNumber> = jsValue.downcast_or_throw::<JsNumber, _>(&mut cx)?;
                let byte: u8 = jsByte.value(&mut cx) as u8;
                slice[i] = byte + 1;
            }
        
            Ok(array)

            */
        }

        method finishMessage(mut cx){
            let payload = cx.argument::<JsString>(0)?.value();
            let payload: Payload = serde_json::from_str(&payload).expect("invalid payload");
            let cb = cx.argument::<JsFunction>(1)?;
            {
                let this = cx.this();
                let guard = cx.lock();
                let ref_ = &(*this.borrow(&guard));
                let client_task = ClientTask {
                    client_id: ref_.client_id.clone(),
                    api: Api::FinishMessage {
                        payload
                    },
                };
                client_task.schedule(cb);
            }

            Ok(cx.undefined().upcast())
        }

        method submit(mut cx) {
            let cb = cx.argument::<JsFunction>(0)?;
            {
                let this = cx.this();
                let guard = cx.lock();
                let ref_ = &(*this.borrow(&guard));
                let client_task = ClientTask {
                    client_id: ref_.client_id.clone(),
                    api: Api::Send {
                        seed: ref_.seed.as_ref().map(|seed| Seed::from_bytes(&hex::decode(&seed).expect("invalid seed hex"))),
                        index: ref_.index.clone(),
                        data: ref_.data.clone(),
                        parents: ref_.parents.clone(),
                        account_index: ref_.account_index,
                        initial_address_index: ref_.initial_address_index,
                        inputs: ref_.inputs.clone(),
                        input_range: ref_.input_range.clone(),
                        outputs: ref_.outputs.clone(),
                        dust_allowance_outputs: ref_.dust_allowance_outputs.clone(),
                    },
                };
                client_task.schedule(cb);
            }

            Ok(cx.undefined().upcast())
        }
    }
}
