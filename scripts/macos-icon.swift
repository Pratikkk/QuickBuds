import AppKit

// A native macOS icon using the same black/white earbud motif as the Android icon.
let size = NSSize(width: 1024, height: 1024)
let image = NSImage(size: size)
image.lockFocus()
NSColor(calibratedWhite: 0.07, alpha: 1).setFill()
NSBezierPath(roundedRect: NSRect(x: 32, y: 32, width: 960, height: 960), xRadius: 210, yRadius: 210).fill()
let symbol = NSImage(systemSymbolName: "earbuds", accessibilityDescription: nil)!
    .withSymbolConfiguration(.init(pointSize: 580, weight: .regular).applying(.init(paletteColors: [.white])))!
let ratio = symbol.size.width / symbol.size.height
let bounds = NSRect(x: (1024 - 580 * ratio) / 2, y: 222, width: 580 * ratio, height: 580)
NSColor.white.set()
symbol.draw(in: bounds)
image.unlockFocus()
let bitmap = NSBitmapImageRep(data: image.tiffRepresentation!)!
try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
