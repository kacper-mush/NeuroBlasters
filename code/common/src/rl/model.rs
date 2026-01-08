use burn::module::{Module, Param};
use burn::nn::{Linear, LinearConfig, Relu};
use burn::tensor::backend::Backend;
use burn::tensor::{Distribution, Tensor};

#[derive(Module, Debug)]
pub struct BotBrain<B: Backend> {
    pub linear1: Linear<B>,
    pub linear2: Linear<B>, // Hidden Layer 1
    pub linear3: Linear<B>, // Hidden Layer 2 (Added for depth)
    pub output: Linear<B>,
    activation: Relu,
}

impl<B: Backend> BotBrain<B> {
    const INPUT_SIZE: usize = super::features::FEATURE_COUNT;
    const OUTPUT_SIZE: usize = 5;

    const HIDDEN_SIZE: usize = 512;

    pub fn new(device: &B::Device) -> Self {
        Self {
            linear1: LinearConfig::new(Self::INPUT_SIZE, Self::HIDDEN_SIZE).init(device),
            linear2: LinearConfig::new(Self::HIDDEN_SIZE, Self::HIDDEN_SIZE).init(device),
            linear3: LinearConfig::new(Self::HIDDEN_SIZE, Self::HIDDEN_SIZE).init(device),
            output: LinearConfig::new(Self::HIDDEN_SIZE, Self::OUTPUT_SIZE).init(device),
            activation: Relu::new(),
        }
    }

    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.linear1.forward(input);
        let x = self.activation.forward(x);

        let x = self.linear2.forward(x);
        let x = self.activation.forward(x);

        // Extra layer processing
        let x = self.linear3.forward(x);
        let x = self.activation.forward(x);

        self.output.forward(x)
    }

    pub fn mutate(&self, power: f32) -> Self {
        let device = self.linear1.weight.device();

        let mutate_param = |param: &Param<Tensor<B, 2>>| -> Param<Tensor<B, 2>> {
            let noise = Tensor::random(param.shape(), Distribution::Normal(0.0, 1.0), &device)
                .mul_scalar(power);
            Param::from_tensor((param.val() + noise).detach())
        };

        let mutate_bias = |param: &Option<Param<Tensor<B, 1>>>| -> Option<Param<Tensor<B, 1>>> {
            if let Some(b) = param {
                let noise = Tensor::random(b.shape(), Distribution::Normal(0.0, 1.0), &device)
                    .mul_scalar(power);
                Some(Param::from_tensor((b.val() + noise).detach()))
            } else {
                None
            }
        };

        Self {
            linear1: Linear {
                weight: mutate_param(&self.linear1.weight),
                bias: mutate_bias(&self.linear1.bias),
            },
            linear2: Linear {
                weight: mutate_param(&self.linear2.weight),
                bias: mutate_bias(&self.linear2.bias),
            },
            linear3: Linear {
                weight: mutate_param(&self.linear3.weight),
                bias: mutate_bias(&self.linear3.bias),
            },
            output: Linear {
                weight: mutate_param(&self.output.weight),
                bias: mutate_bias(&self.output.bias),
            },
            activation: Relu::new(),
        }
    }
}
