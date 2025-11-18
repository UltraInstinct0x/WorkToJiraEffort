#!/usr/bin/env python3
"""Create .icns file for macOS"""

import struct
from PIL import Image
import os

def create_icns(png_path, icns_path):
    """
    Create a simple .icns file from a PNG.
    This is a simplified version that creates a basic icns file.
    """
    img = Image.open(png_path)

    # Common macOS icon sizes
    sizes = [16, 32, 128, 256, 512]

    # ICNS header
    icns_data = b'icns'  # Magic number
    icns_images = []

    # OSType codes for different sizes
    size_codes = {
        16: b'icp4',   # 16x16 icon
        32: b'icp5',   # 32x32 icon
        128: b'icp7',  # 128x128 icon
        256: b'ic08',  # 256x256 icon
        512: b'ic09',  # 512x512 icon
    }

    for size in sizes:
        if size in size_codes:
            resized = img.resize((size, size), Image.Resampling.LANCZOS)

            # Save to PNG format in memory
            import io
            png_buffer = io.BytesIO()
            resized.save(png_buffer, format='PNG')
            png_data = png_buffer.getvalue()

            # Create ICNS entry: type (4 bytes) + size (4 bytes) + data
            entry_size = 8 + len(png_data)
            entry = size_codes[size] + struct.pack('>I', entry_size) + png_data
            icns_images.append(entry)

    # Calculate total size
    total_size = 8 + sum(len(entry) for entry in icns_images)

    # Write ICNS file
    with open(icns_path, 'wb') as f:
        f.write(icns_data)
        f.write(struct.pack('>I', total_size))
        for entry in icns_images:
            f.write(entry)

    print(f"Created {icns_path}")

if __name__ == '__main__':
    create_icns('icons/icon.png', 'icons/icon.icns')
