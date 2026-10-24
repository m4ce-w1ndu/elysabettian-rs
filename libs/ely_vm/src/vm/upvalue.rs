use crate::heap::{HeapObject, ObjUpvalue};
use crate::vm::VM;
use ely_core::value::Value;

impl VM {
    /// Captures a local variable at the given stack location into an open upvalue.
    pub(crate) fn capture_upvalue(&mut self, location: usize) -> usize {
        let existing = self.open_upvalues.iter().copied().find(|&id| {
            matches!(self.heap.get(id), Some(HeapObject::Upvalue(u)) if u.is_open() && u.location == location)
        });

        if let Some(id) = existing {
            return id;
        }

        let id = self.heap.len();
        self.heap
            .push(HeapObject::Upvalue(ObjUpvalue::new(location)));
        self.open_upvalues.push(id);

        id
    }

    /// Closes all open upvalues referencing stack slots at or above `last_slot`.
    pub(crate) fn close_upvalue(&mut self, last_slot: usize) {
        let stack = &self.stack;
        self.open_upvalues.retain(|&id| {
            if let Some(HeapObject::Upvalue(upvalue)) = self.heap.get_mut(id) {
                if upvalue.location >= last_slot {
                    upvalue.closed = Some(stack[upvalue.location]);
                    return false;
                }
            }

            true
        })
    }

    /// Resolves the runtime value of an upvalue captured by a closure.
    pub(crate) fn get_upvalue_value(&self, closure_id: usize, slot: usize) -> Option<Value> {
        let upvalue_id = match self.heap.get(closure_id)? {
            HeapObject::Closure(closure) => *closure.upvalues.get(slot)?,
            _ => return None,
        };

        match self.heap.get(upvalue_id)? {
            HeapObject::Upvalue(upvalue) => {
                if upvalue.is_open() {
                    Some(self.stack[upvalue.location])
                } else {
                    upvalue.closed
                }
            }
            _ => None,
        }
    }

    /// Mutates the value referenced by a closure's upvalue.
    pub(crate) fn set_upvalue_value(
        &mut self,
        closure_id: usize,
        slot: usize,
        value: Value,
    ) -> bool {
        let upvalue_id = match self.heap.get(closure_id) {
            Some(HeapObject::Closure(closure)) => match closure.upvalues.get(slot) {
                Some(&id) => id,
                None => return false,
            },
            _ => return false,
        };

        let location = match self.heap.get(upvalue_id) {
            Some(HeapObject::Upvalue(upvalue)) => {
                if upvalue.is_open() {
                    Some(upvalue.location)
                } else {
                    None
                }
            }
            _ => return false,
        };

        if let Some(loc) = location {
            self.stack[loc] = value;
            true
        } else {
            match self.heap.get_mut(upvalue_id) {
                Some(HeapObject::Upvalue(upvalue)) => {
                    upvalue.closed = Some(value);
                    true
                }
                _ => false,
            }
        }
    }
}
