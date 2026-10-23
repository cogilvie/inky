use rand::Rng;

 use inky::core::colors::Color;


fn main() {
    let eeprom = inky::eeprom::EEPROM::try_new().expect("Failed to initialize eeprom");

    print!("EEPROM initialized successfully!\n {:?} {}x{} {:?}", eeprom.display_variant(), eeprom.width(), eeprom.height(), eeprom.color());
    let mut inky = inky::inky::Inky::try_from(eeprom).expect("Failed to create Inky");

    let canvas = inky.canvas_mut();


    // Draw checkerboard pattern
    let square_size = 10;
    for row in 0..(canvas.height() / square_size) {
        for col in 0..(canvas.width() / square_size) {
            let color = if (row + col) % 2 == 0 { &Color::Black } else { &Color::White };
            let x = col * square_size;
            let y = row * square_size;
            canvas.draw(inky::inky::Rectangle::new((x, y), (x + square_size - 1, y + square_size - 1)), color);
        }
    }

    inky.update().expect("Failed to update display"); 
    print!("Display update complete!\n");
}
