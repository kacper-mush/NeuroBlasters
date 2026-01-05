use burn::module::Module;
use burn::nn::{Linear, LinearConfig, Relu};
use burn::tensor::Tensor;
use burn::tensor::backend::Backend;

#[derive(Module, Debug)]
pub struct BotBrain<B: Backend> {
    linear1: Linear<B>,
    linear2: Linear<B>,
    output: Linear<B>,
    activation: Relu,
}

impl<B: Backend> BotBrain<B> {
    // We'll define specific input/output sizes
    // Input: Features (Health, Enemy Dist, Walls, etc.)
    // Output: (Up, Down, Left, Right, Shoot)
    // TODO: maybe bigger input size (we can add teammates positions or sth)
    const INPUT_SIZE: usize = super::features::FEATURE_COUNT;
    const OUTPUT_SIZE: usize = 5;

    pub fn new(device: &B::Device) -> Self {
        Self {
            linear1: LinearConfig::new(Self::INPUT_SIZE, 64).init(device),
            linear2: LinearConfig::new(64, 64).init(device),
            output: LinearConfig::new(64, Self::OUTPUT_SIZE).init(device),
            activation: Relu::new(),
        }
    }

    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.linear1.forward(input);
        let x = self.activation.forward(x);
        let x = self.linear2.forward(x);
        let x = self.activation.forward(x);
        self.output.forward(x)
    }
}
