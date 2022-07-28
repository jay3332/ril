import ril

def test_create_image() -> None:
    image = ril.Image.new(1, 1, ril.Pixel.from_rgb(255, 255, 255))
    
    assert image.height == 1
    assert image.width == 1
    assert image.dimensions == (1, 1)

def test_image_pixels() -> None:
    image = ril.Image.new(1, 1, ril.Pixel.from_rgb(255, 255, 255))

    assert image.pixels() == [[ril.Rgb(255, 255, 255)]]
