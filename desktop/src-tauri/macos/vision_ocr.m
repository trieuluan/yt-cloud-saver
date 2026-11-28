#import <Foundation/Foundation.h>
#import <CoreGraphics/CoreGraphics.h>
#import <Vision/Vision.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

char *ytcs_vision_recognize_rgb(
    const uint8_t *rgb_ptr,
    uint32_t width,
    uint32_t height,
    const char *langs_csv,
    float min_text_height
) {
    if (!rgb_ptr || width == 0 || height == 0) {
        return strdup("[]");
    }

    @autoreleasepool {
        size_t bytes_per_row = (size_t)width * 3;
        size_t total_bytes = bytes_per_row * (size_t)height;

        CGColorSpaceRef colorSpace = CGColorSpaceCreateDeviceRGB();
        if (!colorSpace) {
            return strdup("[]");
        }

        CFDataRef cfData = CFDataCreate(kCFAllocatorDefault, rgb_ptr, (CFIndex)total_bytes);
        if (!cfData) {
            CGColorSpaceRelease(colorSpace);
            return strdup("[]");
        }

        CGDataProviderRef provider = CGDataProviderCreateWithCFData(cfData);
        CGImageRef cgImage = CGImageCreate(
            (size_t)width,
            (size_t)height,
            8,
            24,
            bytes_per_row,
            colorSpace,
            kCGBitmapByteOrderDefault | kCGImageAlphaNone,
            provider,
            NULL,
            false,
            kCGRenderingIntentDefault
        );

        CGDataProviderRelease(provider);
        CFRelease(cfData);
        CGColorSpaceRelease(colorSpace);

        if (!cgImage) {
            return strdup("[]");
        }

        VNRecognizeTextRequest *request = [[VNRecognizeTextRequest alloc] init];
        request.recognitionLevel = VNRequestTextRecognitionLevelAccurate;
        request.usesLanguageCorrection = YES;
        if (min_text_height > 0.0f && min_text_height < 0.5f) {
            request.minimumTextHeight = min_text_height;
        }

        NSError *langErr = nil;
        NSArray<NSString *> *supported = [request supportedRecognitionLanguagesAndReturnError:&langErr];
        NSSet<NSString *> *supportedSet = supported ? [NSSet setWithArray:supported] : nil;

        if (langs_csv && strlen(langs_csv) > 0) {
            NSString *csvStr = [NSString stringWithUTF8String:langs_csv];
            NSArray<NSString *> *wanted = [csvStr componentsSeparatedByString:@","];
            NSMutableArray<NSString *> *filtered = [NSMutableArray array];
            for (NSString *code in wanted) {
                NSString *trimmed = [code stringByTrimmingCharactersInSet:[NSCharacterSet whitespaceCharacterSet]];
                if (trimmed.length == 0) continue;
                if (!supportedSet || [supportedSet containsObject:trimmed]) {
                    [filtered addObject:trimmed];
                }
            }
            if (filtered.count > 0) {
                request.recognitionLanguages = filtered;
            }
        }
        if (@available(macOS 13.0, *)) {
            request.automaticallyDetectsLanguage = YES;
        }

        VNImageRequestHandler *handler = [[VNImageRequestHandler alloc] initWithCGImage:cgImage options:@{}];
        NSError *execErr = nil;
        BOOL ok = [handler performRequests:@[request] error:&execErr];
        CGImageRelease(cgImage);

        if (!ok || execErr) {
            return strdup("[]");
        }

        NSMutableArray *items = [NSMutableArray array];
        for (VNRecognizedTextObservation *obs in request.results) {
            VNRecognizedText *top = [[obs topCandidates:1] firstObject];
            if (!top || top.string.length == 0) continue;

            CGRect bbox = obs.boundingBox;
            // Convert Vision's bottom-left origin [0..1] to top-left origin [0..1]
            double x = bbox.origin.x;
            double w = bbox.size.width;
            double h = bbox.size.height;
            double y = 1.0 - (bbox.origin.y + h);
            if (y < 0.0) y = 0.0;
            if (x < 0.0) x = 0.0;

            [items addObject:@{
                @"text": top.string,
                @"confidence": @(top.confidence),
                @"x": @(x),
                @"y": @(y),
                @"width": @(w),
                @"height": @(h)
            }];
        }

        NSData *jsonData = [NSJSONSerialization dataWithJSONObject:items options:0 error:nil];
        if (!jsonData) {
            return strdup("[]");
        }
        NSString *jsonStr = [[NSString alloc] initWithData:jsonData encoding:NSUTF8StringEncoding];
        if (!jsonStr) {
            return strdup("[]");
        }
        return strdup([jsonStr UTF8String]);
    }
}

void ytcs_vision_free_str(char *ptr) {
    if (ptr) {
        free(ptr);
    }
}
