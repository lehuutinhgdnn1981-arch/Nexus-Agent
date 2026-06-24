#!/usr/bin/env python3
"""Generate placeholder PNG icons for Tauri."""
import struct
import zlib
import os

def make_png(width, height, color=(99, 102, 241)):
    """Create a minimal PNG with solid color."""
    def chunk(chunk_type, data):
        c = chunk_type + data
        crc = zlib.crc32(c)
        return struct.pack('>I', len(data)) + c + struct.pack('>I', crc)

    sig = b'\x89PNG\r\n\x1a\n'
    ihdr_data = struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0)
    ihdr = chunk(b'IHDR', ihdr_data)
    raw = b''
    for _ in range(height):
        raw += b'\x00'
        for _ in range(width):
            raw += bytes(color)
    compressed = zlib.compress(raw)
    idat = chunk(b'IDAT', compressed)
    iend = chunk(b'IEND', b'')
    return sig + ihdr + idat + iend

out_dir = '/home/z/my-project/nexus/src-tauri/icons'
os.makedirs(out_dir, exist_ok=True)

for size, name in [(32, '32x32.png'), (128, '128x128.png'), (256, '128x128@2x.png')]:
    png = make_png(size, size)
    with open(f'{out_dir}/{name}', 'wb') as f:
        f.write(png)
    print(f'Created {name} ({size}x{size})')

png32 = make_png(32, 32)
ico_header = struct.pack('<HHH', 0, 1, 1)
ico_entry = struct.pack('<BBBBHHII', 32, 32, 0, 0, 1, 32, len(png32), 22)
with open(f'{out_dir}/icon.ico', 'wb') as f:
    f.write(ico_header + ico_entry + png32)
print('Created icon.ico')

icns_header = b'icns'
png128 = make_png(128, 128)
icns_entry = b'ic07' + struct.pack('>I', len(png128) + 8) + png128
total_size = 8 + len(icns_entry)
with open(f'{out_dir}/icon.icns', 'wb') as f:
    f.write(icns_header + struct.pack('>I', total_size) + icns_entry)
print('Created icon.icns')
