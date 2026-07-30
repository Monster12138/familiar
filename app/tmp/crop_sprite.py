import sys
from PIL import Image

def main():
    img_path = 'public/sprites/pixel-cat/sprites.png'
    img = Image.open(img_path).convert("RGBA")
    
    cols, rows = 3, 2
    fw = img.width // cols
    fh = img.height // rows
    
    # Find global min_x, min_y, max_x, max_y across all frames
    min_x, min_y = fw, fh
    max_x, max_y = 0, 0
    
    for r in range(rows):
        for c in range(cols):
            box = (c * fw, r * fh, (c+1)*fw, (r+1)*fh)
            frame = img.crop(box)
            bbox = frame.getbbox()
            if bbox:
                # bbox is (left, upper, right, lower)
                if bbox[0] < min_x: min_x = bbox[0]
                if bbox[1] < min_y: min_y = bbox[1]
                if bbox[2] > max_x: max_x = bbox[2]
                if bbox[3] > max_y: max_y = bbox[3]
                
    if max_x < min_x or max_y < min_y:
        print("Empty image")
        return
        
    # Add a small margin of 2 pixels
    margin = 2
    min_x = max(0, min_x - margin)
    min_y = max(0, min_y - margin)
    max_x = min(fw, max_x + margin)
    max_y = min(fh, max_y + margin)
    
    new_fw = max_x - min_x
    new_fh = max_y - min_y
    
    print(f"Old frame size: {fw}x{fh}")
    print(f"New frame size: {new_fw}x{new_fh}")
    
    new_img = Image.new("RGBA", (new_fw * cols, new_fh * rows), (0, 0, 0, 0))
    
    for r in range(rows):
        for c in range(cols):
            box = (c * fw + min_x, r * fh + min_y, c * fw + max_x, r * fh + max_y)
            frame = img.crop(box)
            new_img.paste(frame, (c * new_fw, r * new_fh))
            
    new_img.save('public/sprites/pixel-cat/sprites_cropped.png')
    print("Saved as sprites_cropped.png")

if __name__ == "__main__":
    main()
