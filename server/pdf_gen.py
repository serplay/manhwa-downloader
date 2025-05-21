import img2pdf
import requests
import os

def generate_chapter_pdf(title, chapter_url):
    # Download images, save them temporarily
    image_paths = []  # save downloaded page images
    # generate PDF
    pdf_path = f"{title}.pdf"
    with open(pdf_path, "wb") as f:
        f.write(img2pdf.convert(image_paths))
    return pdf_path