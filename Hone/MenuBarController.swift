import Cocoa
import SwiftUI
import Carbon

class MenuBarController: NSObject {
    private var statusItem: NSStatusItem!
    private var popover: NSPopover!
    private var hotkeyManager: HotkeyManager?
    
    override init() {
        super.init()
        setupMenuBar()
        setupPopover()
        setupHotkey()
    }
    
    private func setupMenuBar() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        
        if let button = statusItem.button {
            button.image = NSImage(systemSymbolName: "pencil.circle", accessibilityDescription: "Hone")
            button.action = #selector(togglePopover)
            button.target = self
        }
    }
    
    private func setupPopover() {
        popover = NSPopover()
        popover.contentSize = NSSize(width: 450, height: 450)
        popover.behavior = .transient
        popover.contentViewController = NSHostingController(rootView: ContentView())
    }
    
    private func setupHotkey() {
        hotkeyManager = HotkeyManager { [weak self] in
            self?.togglePopover()
        }
    }
    
    @objc func togglePopover() {
        if let button = statusItem.button {
            if popover.isShown {
                closePopover()
            } else {
                popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
                popover.contentViewController?.view.window?.makeKey()
            }
        }
    }
    
    func closePopover() {
        popover.performClose(nil)
    }
}
