use rand::Rng;

 use inky::core::colors::Color;


fn main() {
    let eeprom = inky::eeprom::EEPROM::try_new().expect("Failed to initialize eeprom");

    print!("EEPROM initialized successfully!\n {:?} {}x{} {:?}", eeprom.display_variant(), eeprom.width(), eeprom.height(), eeprom.color());
    let mut inky = inky::inky::Inky::try_from(eeprom).expect("Failed to create Inky");

    let canvas = inky.canvas_mut();

    let mut rng = rand::rng();
    let colors: [Color; 5] = [Color::Red, Color::Green, Color::Blue, Color::Yellow, Color::Black];

    for _ in 0..10 {
        let size = rng.random_range(20..50);
        let x = rng.random_range(0..(canvas.width() - size));
        let y = rng.random_range(0..(canvas.height() - size));
        let color = &colors[rng.random_range(0..colors.len())];

        canvas.draw(inky::inky::Rectangle::new((x, y), (x + size, y + size)), color);
    }

    inky.update().expect("Failed to update display"); 

    let canvas = inky.canvas_mut(); 

    for _ in 0..50 {
        let size = rng.random_range(20..100);
        let x = rng.random_range(0..(canvas.width() - size));
        let y = rng.random_range(0..(canvas.height() - size));
        let color = &colors[rng.random_range(0..colors.len())];

        canvas.draw(inky::inky::Rectangle::new((x, y), (x + size, y + size)), color);
    }

    inky.update().expect("Failed to update display"); 
    print!("Display update complete!\n");
}
