#!/usr/bin/env python3
"""Create .ico file for Windows"""

from PIL import Image

# Load the icon
img = Image.open('icons/icon.png')

# Resize to common Windows icon sizes
sizes = [(16, 16), (32, 32), (48, 48), (256, 256)]
icons = []

for size in sizes:
    resized = img.resize(size, Image.Resampling.LANCZOS)
    icons.append(resized)

# Save as .ico
icons[0].save('icons/icon.ico', format='ICO', sizes=[s for s in sizes])
print("Created icons/icon.ico")
