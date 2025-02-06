// af_nn5a - build a Hebbian neural network using arrayfire as the matrix math backend (see mod neural_net) and wgpu for graphics


use std::time::Instant; // For testing processing times
use pollster::{block_on, FutureExt};

use arrayfire as af;
use af::*;

use mnist::MnistBuilder;
use rand::distributions::{Distribution, Uniform};

mod circuit;
use circuit::*;

mod proc_window;
use proc_window::*;

mod image_proc;
use image_proc::*;

pub const TRAINING_SET_SIZE: usize = 50_000;
pub const VALIDATION_SET_SIZE: usize = 10_000;
pub const TEST_SET_SIZE: usize = 10_000;


fn main() {
    af::set_backend(af::Backend::CUDA);  // Set CUDA backend explicitly
    af::info();  // Get information about the current backend
    
    setup_circuit();
    block_on(run());
}


fn setup_circuit(
    ){

    let now = Instant::now();

    //af::set_backend(af::Backend::CUDA);  // Set CUDA backend explicitly
    //af::info();  // Get information about the current backend
    
    // Create some Layers and put in a Vec<Layers>
    let mut last_column_id: usize = 0;
    let mut last_layer_id: usize = 0;
    let mut column1 = Column::new(1, 1, &mut last_column_id)
                                            .with_layer(Layer::new_square_layer(1, 100,1,1)
                                                            .with_constant_activations(0.02)
                                                            .finalize())
                                            .with_layer(Layer::new_rect_layer(2, 10, 15,1,1)
                                                            .with_constant_activations(0.025)
                                                            .finalize())
                                            .with_connection(1, 2, 1, ConnectionType::Excitatory, 
                                                                &vec![FieldDesc::new(0.0, 0.4, 0.01, 0.1, 0.3, 0.75)
                                                                             ,FieldDesc::new_std(FieldWidth::Immediate)
                                                                             ,FieldDesc::new_std(FieldWidth::LayerWide)
                                                                             ,FieldDesc::new_std(FieldWidth::Direct)])
                                    .finalize();
    println!("Now making Circuit from Column(s)");
    let mut circuit = Circuit::new(1)
                                .with_column(column1)
                                .finalize();
    circuit.override_last_column_id(last_column_id);

    let array_fire_elapsed_time = now.elapsed().as_secs_f64();
    println!("Create model by chained calls in time {}",array_fire_elapsed_time);

}

fn process_circuit() {
    
}

use std::io;
pub fn wait_for_enter() {
    let mut press_continue = String::new();
    println!("Hit Enter to continue");
    io::stdin()
        .read_line(&mut press_continue)
        .expect("Nothing entered");
}

