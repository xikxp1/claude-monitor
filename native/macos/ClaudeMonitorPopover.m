#import <Cocoa/Cocoa.h>
#import <dispatch/dispatch.h>

@interface ClaudeMonitorPopoverDelegate : NSObject <NSPopoverDelegate>
@end

static NSPopover *gPopover = nil;
static ClaudeMonitorPopoverDelegate *gDelegate = nil;
static NSWindow *gSourceWindow = nil;
static NSView *gContentView = nil;
static NSViewController *gContentViewController = nil;
static NSSize gContentSize = {390, 304};

static void restoreContentView(void) {
  if (gSourceWindow != nil && gContentView != nil && gSourceWindow.contentView != gContentView) {
    gSourceWindow.contentView = gContentView;
  }
}

@implementation ClaudeMonitorPopoverDelegate
- (void)popoverDidClose:(NSNotification *)notification {
  restoreContentView();
}
@end

static NSWindow *windowFromPointer(void *windowPtr) {
  id object = (__bridge id)windowPtr;
  if ([object isKindOfClass:[NSWindow class]]) {
    return (NSWindow *)object;
  }
  return nil;
}

static NSStatusItem *statusItemFromTrayPointer(void *trayPtr) {
  id object = (__bridge id)trayPtr;
  if ([object isKindOfClass:[NSStatusItem class]]) {
    return (NSStatusItem *)object;
  }
  if ([object respondsToSelector:@selector(statusItem)]) {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Warc-performSelector-leaks"
    id statusItem = [object performSelector:@selector(statusItem)];
#pragma clang diagnostic pop
    if ([statusItem isKindOfClass:[NSStatusItem class]]) {
      return (NSStatusItem *)statusItem;
    }
  }
  return nil;
}

static NSButton *buttonFromTrayPointer(void *trayPtr) {
  id object = (__bridge id)trayPtr;
  if ([object isKindOfClass:[NSButton class]]) {
    return (NSButton *)object;
  }
  NSStatusItem *statusItem = statusItemFromTrayPointer(trayPtr);
  if ([statusItem respondsToSelector:@selector(button)]) {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Warc-performSelector-leaks"
    id button = [statusItem performSelector:@selector(button)];
#pragma clang diagnostic pop
    if ([button isKindOfClass:[NSButton class]]) {
      return (NSButton *)button;
    }
  }
  return nil;
}

int cm_tray_set_length(void *trayPtr, double length) {
  if (![NSThread isMainThread]) {
    __block int result = 0;
    dispatch_sync(dispatch_get_main_queue(), ^{
      result = cm_tray_set_length(trayPtr, length);
    });
    return result;
  }

  @autoreleasepool {
    @try {
      NSStatusItem *statusItem = statusItemFromTrayPointer(trayPtr);
      if (statusItem == nil || length <= 0) {
        return 0;
      }

      statusItem.length = length;
      return 1;
    } @catch (NSException *exception) {
      NSLog(@"ClaudeMonitorPopover cm_tray_set_length failed: %@", exception);
      return 0;
    }
  }
}

int cm_tray_set_tooltip(void *trayPtr, const char *tooltip) {
  if (![NSThread isMainThread]) {
    __block int result = 0;
    dispatch_sync(dispatch_get_main_queue(), ^{
      result = cm_tray_set_tooltip(trayPtr, tooltip);
    });
    return result;
  }

  @autoreleasepool {
    @try {
      NSButton *button = buttonFromTrayPointer(trayPtr);
      if (button == nil || tooltip == NULL) {
        return 0;
      }

      button.toolTip = [NSString stringWithUTF8String:tooltip];
      return 1;
    } @catch (NSException *exception) {
      NSLog(@"ClaudeMonitorPopover cm_tray_set_tooltip failed: %@", exception);
      return 0;
    }
  }
}

int cm_popover_show(void *windowPtr, void *trayPtr, double width, double height) {
  if (![NSThread isMainThread]) {
    __block int result = 0;
    dispatch_sync(dispatch_get_main_queue(), ^{
      result = cm_popover_show(windowPtr, trayPtr, width, height);
    });
    return result;
  }

  @autoreleasepool {
    @try {
      NSWindow *window = windowFromPointer(windowPtr);
      NSButton *button = buttonFromTrayPointer(trayPtr);

      if (window == nil || button == nil || window.contentView == nil) {
        return 0;
      }

      if (gPopover != nil && gPopover.shown) {
        [gPopover close];
        return 1;
      }

      gSourceWindow = window;
      gContentView = window.contentView;
      gContentSize = NSMakeSize(width, height);

      [window orderOut:nil];

      gContentViewController = [[NSViewController alloc] init];
      gContentViewController.view = gContentView;

      gDelegate = [[ClaudeMonitorPopoverDelegate alloc] init];
      gPopover = [[NSPopover alloc] init];
      gPopover.contentViewController = gContentViewController;
      gPopover.contentSize = gContentSize;
      gPopover.behavior = NSPopoverBehaviorTransient;
      gPopover.animates = YES;
      gPopover.delegate = gDelegate;

      [NSApp activateIgnoringOtherApps:YES];
      [gPopover showRelativeToRect:button.bounds ofView:button preferredEdge:NSRectEdgeMinY];
      [gPopover.contentViewController.view.window makeKeyWindow];

      return 1;
    } @catch (NSException *exception) {
      NSLog(@"ClaudeMonitorPopover cm_popover_show failed: %@", exception);
      restoreContentView();
      return 0;
    }
  }
}

void cm_popover_hide(void) {
  if (![NSThread isMainThread]) {
    dispatch_sync(dispatch_get_main_queue(), ^{
      cm_popover_hide();
    });
    return;
  }

  @autoreleasepool {
    @try {
      if (gPopover != nil && gPopover.shown) {
        [gPopover close];
      } else {
        restoreContentView();
      }
    } @catch (NSException *exception) {
      NSLog(@"ClaudeMonitorPopover cm_popover_hide failed: %@", exception);
      restoreContentView();
    }
  }
}

int cm_popover_is_shown(void) {
  if (![NSThread isMainThread]) {
    __block int result = 0;
    dispatch_sync(dispatch_get_main_queue(), ^{
      result = cm_popover_is_shown();
    });
    return result;
  }

  @autoreleasepool {
    @try {
      return (gPopover != nil && gPopover.shown) ? 1 : 0;
    } @catch (NSException *exception) {
      NSLog(@"ClaudeMonitorPopover cm_popover_is_shown failed: %@", exception);
      return 0;
    }
  }
}
