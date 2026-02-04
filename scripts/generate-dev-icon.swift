#!/usr/bin/env swift

import AppKit
import Foundation

// Generate a dev icon with worktree name badge
// Usage: generate-dev-icon.swift <input-icns> <output-icns> <label>

guard CommandLine.arguments.count == 4 else {
    fputs("Usage: \(CommandLine.arguments[0]) <input-icns> <output-icns> <label>\n", stderr)
    exit(1)
}

let inputPath = CommandLine.arguments[1]
let outputPath = CommandLine.arguments[2]
let label = CommandLine.arguments[3]

// Load the icns file
guard let iconImage = NSImage(contentsOfFile: inputPath) else {
    fputs("Failed to load image: \(inputPath)\n", stderr)
    exit(1)
}

// Get the largest representation for best quality
guard let rep = iconImage.representations.max(by: { $0.pixelsWide < $1.pixelsWide }) else {
    fputs("No image representations found\n", stderr)
    exit(1)
}

let size = NSSize(width: rep.pixelsWide, height: rep.pixelsHigh)

// Create a new image with the badge
let newImage = NSImage(size: size)
newImage.lockFocus()

// Draw the original icon
iconImage.draw(in: NSRect(origin: .zero, size: size))

// Configure badge
let badgeHeight = size.height * 0.22
let padding = size.width * 0.03
let cornerRadius = badgeHeight * 0.25

// Text attributes - calculate font size to fit
let maxFontSize = badgeHeight * 0.65
var fontSize = maxFontSize
var textSize: NSSize
var attributes: [NSAttributedString.Key: Any]

// Find font size that fits
repeat {
    attributes = [
        .font: NSFont.systemFont(ofSize: fontSize, weight: .bold),
        .foregroundColor: NSColor.white
    ]
    textSize = (label as NSString).size(withAttributes: attributes)
    fontSize -= 1
} while textSize.width > size.width * 0.9 && fontSize > 8

// Badge dimensions based on text
let badgeWidth = textSize.width + padding * 4
let badgeX = size.width - badgeWidth - padding
let badgeY = padding + size.height * 0.05  // Raised slightly higher

// Draw badge background (dark semi-transparent, no border)
let badgePath = NSBezierPath(roundedRect: NSRect(x: badgeX, y: badgeY, width: badgeWidth, height: badgeHeight), 
                              xRadius: cornerRadius, yRadius: cornerRadius)
NSColor(calibratedRed: 0.1, green: 0.1, blue: 0.1, alpha: 0.85).setFill()
badgePath.fill()

// Draw text centered in badge
let textX = badgeX + (badgeWidth - textSize.width) / 2
let textY = badgeY + (badgeHeight - textSize.height) / 2
(label as NSString).draw(at: NSPoint(x: textX, y: textY), withAttributes: attributes)

newImage.unlockFocus()

// Convert to icns format
// First, create PNG data at multiple sizes for icns
guard let tiffData = newImage.tiffRepresentation,
      let bitmapRep = NSBitmapImageRep(data: tiffData) else {
    fputs("Failed to create bitmap representation\n", stderr)
    exit(1)
}

// For simplicity, we'll create a PNG and then use iconutil if available,
// or just save as PNG for the icon (Tauri can use PNG)
guard let pngData = bitmapRep.representation(using: .png, properties: [:]) else {
    fputs("Failed to create PNG data\n", stderr)
    exit(1)
}

// If output is .icns, we need to create an iconset and use iconutil
if outputPath.hasSuffix(".icns") {
    let tempDir = FileManager.default.temporaryDirectory
    let iconsetPath = tempDir.appendingPathComponent("staged-dev.iconset")
    
    // Remove existing iconset if present
    try? FileManager.default.removeItem(at: iconsetPath)
    try! FileManager.default.createDirectory(at: iconsetPath, withIntermediateDirectories: true)
    
    // Generate all required sizes for iconset
    let sizes: [(name: String, size: Int)] = [
        ("icon_16x16", 16),
        ("icon_16x16@2x", 32),
        ("icon_32x32", 32),
        ("icon_32x32@2x", 64),
        ("icon_128x128", 128),
        ("icon_128x128@2x", 256),
        ("icon_256x256", 256),
        ("icon_256x256@2x", 512),
        ("icon_512x512", 512),
        ("icon_512x512@2x", 1024)
    ]
    
    for (name, targetSize) in sizes {
        let resizedImage = NSImage(size: NSSize(width: targetSize, height: targetSize))
        resizedImage.lockFocus()
        NSGraphicsContext.current?.imageInterpolation = .high
        newImage.draw(in: NSRect(x: 0, y: 0, width: targetSize, height: targetSize))
        resizedImage.unlockFocus()
        
        guard let resizedTiff = resizedImage.tiffRepresentation,
              let resizedBitmap = NSBitmapImageRep(data: resizedTiff),
              let resizedPng = resizedBitmap.representation(using: .png, properties: [:]) else {
            continue
        }
        
        let filePath = iconsetPath.appendingPathComponent("\(name).png")
        try! resizedPng.write(to: filePath)
    }
    
    // Use iconutil to create icns
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
    process.arguments = ["-c", "icns", iconsetPath.path, "-o", outputPath]
    try! process.run()
    process.waitUntilExit()
    
    // Cleanup
    try? FileManager.default.removeItem(at: iconsetPath)
    
    if process.terminationStatus != 0 {
        fputs("iconutil failed\n", stderr)
        exit(1)
    }
} else {
    // Just save as PNG
    try! pngData.write(to: URL(fileURLWithPath: outputPath))
}

print("Generated: \(outputPath)")
