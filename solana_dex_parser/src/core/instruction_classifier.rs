use std::collections::{HashMap, HashSet};

use crate::core::transaction_adapter::TransactionAdapter;
use crate::types::ClassifiedInstruction;

#[derive(Clone, Debug)]
pub struct InstructionClassifier {
    instructions_by_program: HashMap<String, Vec<ClassifiedInstruction>>,
    order: Vec<String>,
}

impl InstructionClassifier {
    pub fn new(adapter: &TransactionAdapter) -> Self {
        let mut instructions_by_program: HashMap<String, Vec<ClassifiedInstruction>> =
            HashMap::new();
        let mut order: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for (index, instruction) in adapter.instructions().iter().cloned().enumerate() {
            let program_id = instruction.program_id.clone();
            let classified = ClassifiedInstruction {
                program_id: program_id.clone(),
                outer_index: index,
                inner_index: None,
                data: instruction,
            };
            instructions_by_program
                .entry(program_id.clone())
                .or_default()
                .push(classified);
            if seen.insert(program_id.clone()) {
                order.push(program_id);
            }
        }

        Self {
            instructions_by_program,
            order,
        }
    }

    pub fn get_all_program_ids(&self) -> Vec<String> {
        self.order.clone()
    }

    pub fn get_instructions(&self, program_id: &str) -> Vec<ClassifiedInstruction> {
        self.instructions_by_program
            .get(program_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn flatten(&self) -> Vec<ClassifiedInstruction> {
        self.instructions_by_program
            .values()
            .flatten()
            .cloned()
            .collect()
    }
}
