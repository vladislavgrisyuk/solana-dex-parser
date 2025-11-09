use std::collections::{HashMap, HashSet};

use crate::core::transaction_adapter::TransactionAdapter;
use crate::types::ClassifiedInstruction;

// ← Подправь путь при необходимости
use crate::constants::{SKIP_PROGRAM_IDS, SYSTEM_PROGRAMS};
// Утилита, аналогичная TS getInstructionData(instruction)
// Должна вернуть байты data инструкции.
// Подправь путь/сигнатуру при необходимости.
use crate::core::utils::get_instruction_data;

#[derive(Clone, Debug)]
pub struct InstructionClassifier {
    instruction_map: HashMap<String, Vec<ClassifiedInstruction>>,
    // храним порядок «первого появления» program_id (как в TS порядок ключей Map)
    order: Vec<String>,
}

impl InstructionClassifier {
    pub fn new(adapter: &TransactionAdapter) -> Self {
        let mut instruction_map: HashMap<String, Vec<ClassifiedInstruction>> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        // OUTER instructions
        for (outer_index, instruction) in adapter.instructions().iter().cloned().enumerate() {
            let program_id = instruction.program_id.clone();
            if program_id.is_empty() {
                continue;
            }
            let classified = ClassifiedInstruction {
                program_id: program_id.clone(),
                outer_index,
                inner_index: None,
                data: instruction,
            };
            instruction_map
                .entry(program_id.clone())
                .or_default()
                .push(classified);
            if seen.insert(program_id.clone()) {
                order.push(program_id);
            }
        }

        // INNER instructions
        for inner in adapter.inner_instructions() {
            for (inner_index, instruction) in inner.instructions.iter().cloned().enumerate() {
                let program_id = instruction.program_id.clone();
                if program_id.is_empty() {
                    continue;
                }
                let classified = ClassifiedInstruction {
                    program_id: program_id.clone(),
                    outer_index: inner.index,
                    inner_index: Some(inner_index),
                    data: instruction,
                };
                instruction_map
                    .entry(program_id.clone())
                    .or_default()
                    .push(classified);
                if seen.insert(program_id.clone()) {
                    order.push(program_id);
                }
            }
        }

        Self {
            instruction_map,
            order,
        }
    }

    /// Полный список program_id в порядке первого появления,
    /// но с фильтром как в TS: исключаем системные и «skip».
    pub fn get_all_program_ids(&self) -> Vec<String> {
        self.order
            .iter()
            .cloned()
            .filter(|pid| {
                let pid_str = pid.as_str();
                !SYSTEM_PROGRAMS.contains(&pid_str) && !SKIP_PROGRAM_IDS.contains(&pid_str)
            })
            .collect()
    }

    /// Все инструкции по одному program_id
    pub fn get_instructions(&self, program_id: &str) -> Vec<ClassifiedInstruction> {
        self.instruction_map
            .get(program_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Инструкции по нескольким program_id (flatten), как getMultiInstructions в TS
    pub fn get_multi_instructions<S: AsRef<str>>(
        &self,
        program_ids: &[S],
    ) -> Vec<ClassifiedInstruction> {
        let mut out = Vec::new();
        for pid in program_ids {
            if let Some(vec) = self.instruction_map.get(pid.as_ref()) {
                out.extend(vec.iter().cloned());
            }
        }
        out
    }

    /// Поиск инструкции по дискриминатору (первые `slice` байт)
    /// Полный аналог TS: getInstructionByDescriminator(Buffer, slice)
    pub fn get_instruction_by_discriminator(
        &self,
        discriminator: &[u8],
        slice: usize,
    ) -> Option<ClassifiedInstruction> {
        for instructions in self.instruction_map.values() {
            for ci in instructions {
                // get_instruction_data должен вернуть &[u8] / Vec<u8> с реальными байтами data
                let data = get_instruction_data(&ci.data);
                if data.len() >= slice && &data[..slice] == discriminator {
                    return Some(ci.clone());
                }
            }
        }
        None
    }

    /// Опционально оставил (в TS нет, но вдруг пригодится)
    pub fn flatten(&self) -> Vec<ClassifiedInstruction> {
        self.instruction_map.values().flatten().cloned().collect()
    }
}
