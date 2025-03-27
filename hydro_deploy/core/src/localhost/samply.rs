use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

use futures::future::join_all;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Number;
use wholesym::debugid::DebugId;
use wholesym::{LookupAddress, MultiArchDisambiguator, SymbolManager, SymbolManagerConfig};

#[derive(Serialize, Deserialize, Debug)]
pub struct FxProfile {
    threads: Vec<Thread>,
    libs: Vec<Lib>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Lib {
    pub path: String,
    #[serde(rename = "breakpadId")]
    pub breakpad_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Thread {
    #[serde(rename = "stackTable")]
    pub stack_table: StackTable,
    #[serde(rename = "frameTable")]
    pub frame_table: FrameTable,
    #[serde(rename = "funcTable")]
    pub func_table: FuncTable,
    pub samples: Samples,
    #[serde(rename = "isMainThread")]
    pub is_main_thread: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Samples {
    pub stack: Vec<Option<usize>>,
    pub weight: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StackTable {
    pub prefix: Vec<Option<usize>>,
    pub frame: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FrameTable {
    /// `Vec` of `u64` or `-1`.
    pub address: Vec<Number>,
    pub func: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FuncTable {
    // `Vec` of `usize` or `-1`.
    pub resource: Vec<Number>,
}

pub async fn samply_to_folded(loaded: FxProfile) -> String {
    let symbol_manager = SymbolManager::with_config(SymbolManagerConfig::default());

    let mut symbol_maps = vec![];
    for lib in &loaded.libs {
        symbol_maps.push(
            symbol_manager
                .load_symbol_map_for_binary_at_path(
                    &PathBuf::from_str(&lib.path).unwrap(),
                    Some(MultiArchDisambiguator::DebugId(
                        DebugId::from_breakpad(&lib.breakpad_id).unwrap(),
                    )),
                )
                .await
                .ok(),
        );
    }

    let mut folded_frames: BTreeMap<Vec<Option<String>>, u64> = BTreeMap::new();
    for thread in loaded.threads.into_iter().filter(|t| t.is_main_thread) {
        let frame_lookuped = join_all((0..thread.frame_table.address.len()).map(|frame_id| {
            let fr_address = &thread.frame_table.address;
            let fr_func = &thread.frame_table.func;
            let fn_resource = &thread.func_table.resource;
            let symbol_maps = &symbol_maps;
            async move {
                let address = fr_address[frame_id].as_u64()?;
                let func_id = fr_func[frame_id];
                let resource_id = fn_resource[func_id].as_u64()?;
                let symbol_map = symbol_maps[resource_id as usize].as_ref()?;
                let lookuped = symbol_map
                    .lookup(LookupAddress::Relative(address as u32))
                    .await?;

                if let Some(inline_frames) = lookuped.frames {
                    Some(
                        inline_frames
                            .into_iter()
                            .rev()
                            .map(|inline| inline.function.unwrap_or_else(|| "unknown".to_string()))
                            .join(";"),
                    )
                } else {
                    Some(lookuped.symbol.name)
                }
            }
        }))
        .await;

        let all_leaves_grouped = thread
            .samples
            .stack
            .iter()
            .enumerate()
            .filter_map(|(idx, s)| s.map(|s| (idx, s)))
            .map(|(idx, leaf)| (leaf, thread.samples.weight[idx]))
            .chunk_by(|&(leaf, _)| leaf)
            .into_iter()
            .map(|(leaf, group)| {
                let weight = group.map(|(_leaf, weight)| weight).sum();
                (leaf, weight)
            })
            .collect::<Vec<(usize, u64)>>();

        for (leaf, weight) in all_leaves_grouped {
            let mut cur_stack = Some(leaf);
            let mut stack = vec![];
            while let Some(sample) = cur_stack {
                let frame_id = thread.stack_table.frame[sample];
                stack.push(frame_lookuped[frame_id].clone());
                cur_stack = thread.stack_table.prefix[sample];
            }

            *folded_frames.entry(stack).or_default() += weight;
        }
    }

    let mut output = String::new();
    for (stack, weight) in folded_frames {
        for (i, s) in stack.iter().rev().enumerate() {
            if i != 0 {
                output.push(';');
            }
            output.push_str(s.as_deref().unwrap_or("unknown"));
        }

        output.push_str(&format!(" {}\n", weight));
    }

    output
}
