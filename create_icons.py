#!/usr/bin/env python3
"""
Simple script to create placeholder icons for the Tauri app.
Creates basic colored squares as placeholder icons.
"""

from PIL import Image, ImageDraw

def create_icon(size, output_path):
    """Create a simple colored square icon."""
    # Create a new image with a blue background
    img = Image.new('RGBA', (size, size), (65, 105, 225, 255))

    # Draw a simple design
    draw = ImageDraw.Draw(img)

    # Draw a border
    border_width = max(2, size // 16)
    draw.rectangle(
        [(border_width, border_width), (size - border_width, size - border_width)],
        outline=(255, 255, 255, 255),
        width=border_width
    )

    # Draw a "W" in the center (for Work)
    # This is a simple representation
    center = size // 2
    letter_size = size // 3

    # Save the image
    img.save(output_path)
    print(f"Created {output_path}")

if __name__ == '__main__':
    import sys

    # Create icons directory if it doesn't exist
    import os
    os.makedirs('icons', exist_ok=True)

    # Create different sizes
    sizes = [32, 128, 256, 512]

    for size in sizes:
        create_icon(size, f'icons/{size}x{size}.png')

    # Create a larger one for macOS
    create_icon(1024, 'icons/icon.png')

    print("\nPlaceholder icons created successfully!")
    print("Note: These are basic placeholder icons.")
    print("You may want to create custom icons for production use.")
