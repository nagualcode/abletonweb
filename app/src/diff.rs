//! Minimal structural diff of JSON states: replace ops for changed leaves.
//! Arrays whose length changed are replaced whole, keeping patches
//! index-stable. Mirrors applyPatch in web/src/lib/live.svelte.ts.

use serde_json::Value;

pub fn diff(prev: &Value, next: &Value) -> Vec<Value> {
    let mut ops = Vec::new();
    walk(prev, next, &mut Vec::new(), &mut ops);
    ops
}

fn walk(prev: &Value, next: &Value, path: &mut Vec<Value>, ops: &mut Vec<Value>) {
    if prev == next {
        return;
    }
    match (prev, next) {
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                path.push(i.into());
                walk(x, y, path, ops);
                path.pop();
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            for (k, y) in b {
                path.push(k.clone().into());
                walk(a.get(k).unwrap_or(&Value::Null), y, path, ops);
                path.pop();
            }
            for k in a.keys().filter(|k| !b.contains_key(*k)) {
                path.push(k.clone().into());
                ops.push(Value::Array(vec![Value::Array(path.clone()), Value::Null]));
                path.pop();
            }
        }
        _ => ops.push(Value::Array(vec![Value::Array(path.clone()), next.clone()])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn apply(mut root: Value, ops: &[Value]) -> Value {
        for op in ops {
            let path = op[0].as_array().unwrap();
            let value = op[1].clone();
            if path.is_empty() {
                root = value;
                continue;
            }
            let mut node = &mut root;
            for key in &path[..path.len() - 1] {
                node = match key {
                    Value::Number(n) => &mut node[n.as_u64().unwrap() as usize],
                    Value::String(s) => &mut node[s.as_str()],
                    _ => unreachable!(),
                };
            }
            match &path[path.len() - 1] {
                Value::Number(n) => node[n.as_u64().unwrap() as usize] = value,
                Value::String(s) => node[s.as_str()] = value,
                _ => unreachable!(),
            }
        }
        root
    }

    #[test]
    fn round_trips() {
        let a = json!({"song": {"tempo": 120, "isPlaying": false}, "tracks": [{"name": "a", "slots": [1, 2]}, {"name": "b"}]});
        let b = json!({"song": {"tempo": 121, "isPlaying": false}, "tracks": [{"name": "a", "slots": [1, 2, 3]}, {"name": "c"}]});
        let ops = diff(&a, &b);
        assert_eq!(ops.len(), 3);
        assert_eq!(apply(a.clone(), &ops), b);
        assert!(diff(&b, &b).is_empty());
        let c = json!({"song": {"tempo": 121}, "tracks": []});
        assert_eq!(apply(b.clone(), &diff(&b, &c))["tracks"], json!([]));
    }
}
