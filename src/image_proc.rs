// image_proc.rs - support functions for processing the MNIST hand-written numerals image data
//
use mnist::Mnist;
pub const IMAGE_DIMENSION: usize = 784;
pub const ACTUAL_IMAGE_DIMENSION: usize = 784;

use arrayfire as af;
use af::*;


use num_integer::Roots;
fn create_center_image(image: &Array<f32>) -> Array<f32> {
    // Create an n x n (where n < sqrt(ACTUAL_IMAGE_DIMENSION))  'skip' array out of the image that is centered on the center of the image and evenly skipping pixels
    //   This is done to reduce the number of parameters in the model
    let new_image_row_col_size = IMAGE_DIMENSION.sqrt() as i32; // assumes image_dimension is a perfect square <= image.dims()[0]
    let old_image_row_col_size = ACTUAL_IMAGE_DIMENSION.sqrt() as i32;
    let optimal_skip_size = old_image_row_col_size / new_image_row_col_size;
    
    let center_orig_image = (old_image_row_col_size /2) - 1;
    let center_new_image = (new_image_row_col_size /2) - 1;
    let start_row_col = center_orig_image - center_new_image * optimal_skip_size;
    
    //println!("Loading center image of {} x {} from orig image of {} x {} and skipping every {} pixels", 
    //    new_image_row_col_size, new_image_row_col_size, old_image_row_col_size, old_image_row_col_size, optimal_skip_size);
    
    assert!(new_image_row_col_size <= old_image_row_col_size);
    let redim_image  = af::moddims(&image, Dim4::new(&[28,28,1,1])); // the original image is [1,784, 1,1] but should be used as [28,28,1,1]
    let row_index = af::Seq::new(start_row_col, start_row_col + (new_image_row_col_size - 1) * optimal_skip_size, optimal_skip_size); // Both were optimal_skip_size 
    let col_index = af::Seq::new(start_row_col, start_row_col + (new_image_row_col_size - 1)* optimal_skip_size, optimal_skip_size); 

    let reassign_seq = [row_index, col_index, Seq::default(), Seq::default()];
    //println!("Assigning center image using sequence {:?} ", &reassign_seq) ;
    //af_print!("From redim_image", &redim_image);
    let center_image = af::moddims(&af::index(&redim_image, &reassign_seq),Dim4::new(&[1,IMAGE_DIMENSION as u64,1,1])); 
    //af_print!("To center_image after index", &center_image);
    //wait_for_enter();
    center_image
}

