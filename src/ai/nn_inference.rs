//! Simple MLP inference in pure Rust.
//!
//! Loads trained PyTorch weights from binary file and runs forward pass.
//! Used by MCTS to get NN policy (action priorities) and value (state evaluation)
//! without needing Python.
//!
//! Binary weight format (little-endian):
//!   u32: num_layers
//!   For each layer: u32 rows, u32 cols, f32[rows*cols] weights, f32[rows] bias
//!   Then: policy head (rows=ACT_DIM, cols=last_hidden)
//!   Then: value head (rows=1, cols=last_hidden)

/// A dense layer: y = W*x + b, then optional ReLU.
#[derive(Clone)]
struct DenseLayer {
    weights: Vec<f32>,  // [rows × cols], row-major
    bias: Vec<f32>,     // [rows]
    rows: usize,
    cols: usize,
}

impl DenseLayer {
    fn forward(&self, input: &[f32], output: &mut [f32], relu: bool) {
        assert_eq!(input.len(), self.cols);
        assert_eq!(output.len(), self.rows);
        for r in 0..self.rows {
            let mut sum = self.bias[r];
            let row_start = r * self.cols;
            for c in 0..self.cols {
                sum += self.weights[row_start + c] * input[c];
            }
            output[r] = if relu { sum.max(0.0) } else { sum };
        }
    }
}

/// Simple MLP with shared backbone + policy head + value head.
#[derive(Clone)]
pub struct SimpleMLP {
    shared: Vec<DenseLayer>,
    policy_head: DenseLayer,
    value_head: DenseLayer,
}

impl SimpleMLP {
    /// Load weights from binary file.
    pub fn load(path: &str) -> Result<Self, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read weights file '{}': {}", path, e))?;
        
        if data.len() < 4 {
            return Err("Weight file too small".into());
        }
        
        let mut offset = 0;
        
        let read_u32 = |data: &[u8], off: &mut usize| -> Result<u32, String> {
            if *off + 4 > data.len() {
                return Err(format!("Unexpected EOF at offset {}", off));
            }
            let val = u32::from_le_bytes([data[*off], data[*off+1], data[*off+2], data[*off+3]]);
            *off += 4;
            Ok(val)
        };
        
        let read_f32_vec = |data: &[u8], off: &mut usize, n: usize| -> Result<Vec<f32>, String> {
            let bytes = n * 4;
            if *off + bytes > data.len() {
                return Err(format!("EOF reading {} floats at offset {}", n, off));
            }
            let mut v = Vec::with_capacity(n);
            for i in 0..n {
                let idx = *off + i * 4;
                let val = f32::from_le_bytes([data[idx], data[idx+1], data[idx+2], data[idx+3]]);
                v.push(val);
            }
            *off += bytes;
            Ok(v)
        };
        
        let read_layer = |data: &[u8], off: &mut usize| -> Result<DenseLayer, String> {
            let rows = read_u32(data, off)? as usize;
            let cols = read_u32(data, off)? as usize;
            let weights = read_f32_vec(data, off, rows * cols)?;
            let bias = read_f32_vec(data, off, rows)?;
            Ok(DenseLayer { weights, bias, rows, cols })
        };
        
        // Read shared layers
        let n_shared = read_u32(&data, &mut offset)? as usize;
        let mut shared = Vec::with_capacity(n_shared);
        for _ in 0..n_shared {
            shared.push(read_layer(&data, &mut offset)?);
        }
        
        // Read policy head
        let policy_head = read_layer(&data, &mut offset)?;
        
        // Read value head
        let value_head = read_layer(&data, &mut offset)?;
        
        Ok(SimpleMLP { shared, policy_head, value_head })
    }
    
    /// Forward pass: returns (policy_logits, value).
    pub fn forward(&self, input: &[f32]) -> (Vec<f32>, f32) {
        let mut current = input.to_vec();
        let mut next;
        
        // Shared layers with ReLU
        for layer in &self.shared {
            next = vec![0.0; layer.rows];
            layer.forward(&current, &mut next, true);
            current = next;
        }
        
        // Policy head (no activation — raw logits)
        let mut policy = vec![0.0; self.policy_head.rows];
        self.policy_head.forward(&current, &mut policy, false);
        
        // Value head (no activation — raw value)
        let mut value_out = vec![0.0; 1];
        self.value_head.forward(&current, &mut value_out, false);
        
        (policy, value_out[0])
    }
    
    /// Get action probabilities (softmax over valid actions).
    pub fn get_policy_probs(&self, obs: &[f32], valid_actions: &[i32]) -> Vec<(i32, f32)> {
        let (logits, _) = self.forward(obs);
        
        // Softmax over valid actions only
        let mut max_logit = f32::NEG_INFINITY;
        for &a in valid_actions {
            let l = logits[a as usize];
            if l > max_logit { max_logit = l; }
        }
        
        let mut sum = 0.0f32;
        let mut probs: Vec<(i32, f32)> = Vec::with_capacity(valid_actions.len());
        for &a in valid_actions {
            let p = (logits[a as usize] - max_logit).exp();
            probs.push((a, p));
            sum += p;
        }
        
        if sum > 0.0 {
            for p in probs.iter_mut() {
                p.1 /= sum;
            }
        }
        
        probs
    }
    
    /// Sample an action from the policy distribution.
    pub fn sample_action(&self, obs: &[f32], valid_actions: &[i32], temperature: f32, rng_val: f64) -> i32 {
        let (logits, _) = self.forward(obs);
        
        // Temperature-scaled softmax
        let mut max_logit = f32::NEG_INFINITY;
        for &a in valid_actions {
            let l = logits[a as usize] / temperature.max(0.01);
            if l > max_logit { max_logit = l; }
        }
        
        let mut cumulative = Vec::with_capacity(valid_actions.len());
        let mut sum = 0.0f32;
        for &a in valid_actions {
            let p = (logits[a as usize] / temperature.max(0.01) - max_logit).exp();
            sum += p;
            cumulative.push((a, sum));
        }
        
        let threshold = rng_val as f32 * sum;
        for &(a, cum) in &cumulative {
            if threshold <= cum {
                return a;
            }
        }
        
        *valid_actions.last().unwrap_or(&10)
    }
    
    /// Get value estimate for a state.
    pub fn get_value(&self, obs: &[f32]) -> f32 {
        let (_, value) = self.forward(obs);
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dense_layer_forward() {
        let layer = DenseLayer {
            weights: vec![1.0, 2.0, 3.0, 4.0], // 2×2
            bias: vec![0.5, -0.5],
            rows: 2, cols: 2,
        };
        let input = vec![1.0, 1.0];
        let mut output = vec![0.0; 2];
        layer.forward(&input, &mut output, false);
        assert!((output[0] - 3.5).abs() < 1e-6); // 1*1 + 2*1 + 0.5
        assert!((output[1] - 6.5).abs() < 1e-6); // 3*1 + 4*1 - 0.5
    }
    
    #[test]
    fn test_dense_layer_relu() {
        let layer = DenseLayer {
            weights: vec![-1.0, -1.0],
            bias: vec![-1.0],
            rows: 1, cols: 2,
        };
        let input = vec![1.0, 1.0];
        let mut output = vec![0.0; 1];
        layer.forward(&input, &mut output, true);
        assert_eq!(output[0], 0.0); // relu(-3) = 0
    }
}
