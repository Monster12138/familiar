import Cocoa

let options = CGWindowListOption.optionOnScreenOnly
let windowListInfo = CGWindowListCopyWindowInfo(options, CGWindowID(0))
guard let infoList = windowListInfo as NSArray? as? [[String: AnyObject]] else { exit(1) }

for info in infoList {
    if let owner = info["kCGWindowOwnerName"] as? String, owner == "familiar-app" {
        if let windowID = info["kCGWindowNumber"] as? NSNumber {
            print(windowID.intValue)
            exit(0)
        }
    }
}
print("NOT_FOUND")
