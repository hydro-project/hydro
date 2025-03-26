use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
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
    pub address: Vec<u64>,
    pub func: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FuncTable {
    pub resource: Vec<usize>,
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

    let mut folded_frames: BTreeMap<Vec<String>, u64> = BTreeMap::new();
    for thread in loaded.threads.into_iter().filter(|t| t.is_main_thread) {
        let mut frame_lookuped = vec![];
        for frame_id in 0..thread.frame_table.address.len() {
            let address = thread.frame_table.address[frame_id];
            let func_id = thread.frame_table.func[frame_id];
            let resource_id = thread.func_table.resource[func_id];
            let maybe_symbol_map = &symbol_maps[resource_id];

            if let Some(symbols_map) = maybe_symbol_map {
                if let Some(lookuped) = symbols_map
                    .lookup(LookupAddress::Relative(address as u32))
                    .await
                {
                    if let Some(inline_frames) = lookuped.frames {
                        frame_lookuped.push(
                            inline_frames
                                .into_iter()
                                .rev()
                                .map(|inline| {
                                    inline.function.unwrap_or_else(|| "unknown".to_string())
                                })
                                .join(";"),
                        );
                    } else {
                        frame_lookuped.push(lookuped.symbol.name);
                    }
                } else {
                    frame_lookuped.push("unknown".to_string());
                }
            } else {
                frame_lookuped.push("unknown".to_string());
            }
        }

        let all_leaves_grouped = thread
            .samples
            .stack
            .iter()
            .enumerate()
            .filter_map(|(idx, s)| s.map(|s| (idx, s)))
            .map(|(idx, leaf)| (leaf, thread.samples.weight[idx]))
            .chunk_by(|v| v.0)
            .into_iter()
            .map(|(leaf, group)| {
                let weight = group.map(|t| t.1).sum();
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
            output.push_str(s);
        }

        output.push_str(&format!(" {}\n", weight));
    }

    output
}
