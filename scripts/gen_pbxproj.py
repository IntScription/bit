#!/usr/bin/env python3
"""Generates ios/BitApp/BitApp.xcodeproj/project.pbxproj.

Hand-written pbxproj generation, kept as a script (not a one-off) so the
project can be regenerated if the file list changes, rather than hand-
edited. Run from anywhere; paths below are relative to this script.
"""
import uuid
import os

def uid():
    return uuid.uuid4().hex[:24].upper()

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROJECT_DIR = os.path.join(ROOT, "ios", "BitApp")

app_sources = [
    "BitApp.swift",
    "Engine.swift",
    "SplashView.swift",
    "RootView.swift",
    "Theme.swift",
    "Haptics.swift",
    "Components.swift",
    "Demos.swift",
    "ExploreView.swift",
    "PlayView.swift",
    "BuildView.swift",
    "LabView.swift",
    "ProfileView.swift",
]

uitest_sources = ["BitAppUITests.swift"]

ids = {}
for name in [
    "project", "main_group", "products_group", "app_group",
    "app_target", "app_product",
    "generated_group", "bit_ffi_swift_ref", "bit_ffi_swift_build",
    "xcframework_ref", "xcframework_build",
    "content_ref", "content_build",
    "assets_ref", "assets_build",
    "sources_phase", "frameworks_phase", "resources_phase",
    "proj_config_list", "proj_debug", "proj_release",
    "target_config_list", "target_debug", "target_release",
    # UI test target
    "uitests_group", "uitests_target", "uitests_product",
    "uitests_sources_phase", "uitests_frameworks_phase", "uitests_resources_phase",
    "uitests_config_list", "uitests_debug", "uitests_release",
    "uitests_container_proxy", "uitests_dependency",
]:
    ids[name] = uid()

for name in app_sources:
    ids[f"ref_{name}"] = uid()
    ids[f"build_{name}"] = uid()

for name in uitest_sources:
    ids[f"ref_{name}"] = uid()
    ids[f"build_{name}"] = uid()

def swift_ref(name):
    return (
        f'\t\t{ids[f"ref_{name}"]} /* {name} */ = {{isa = PBXFileReference; '
        f'lastKnownFileType = sourcecode.swift; path = {name}; sourceTree = "<group>"; }};'
    )

def swift_build(name):
    return f'\t\t{ids[f"build_{name}"]} /* {name} in Sources */ = {{isa = PBXBuildFile; fileRef = {ids[f"ref_{name}"]} /* {name} */; }};'

file_refs = "\n".join(swift_ref(n) for n in app_sources)
build_files_sources = "\n".join(swift_build(n) for n in app_sources)
group_children = "\n".join(f'\t\t\t\t{ids[f"ref_{n}"]} /* {n} */,' for n in app_sources)
sources_phase_files = "\n".join(f'\t\t\t\t{ids[f"build_{n}"]} /* {n} in Sources */,' for n in app_sources)

uitests_file_refs = "\n".join(swift_ref(n) for n in uitest_sources)
uitests_build_files_sources = "\n".join(swift_build(n) for n in uitest_sources)
uitests_group_children = "\n".join(f'\t\t\t\t{ids[f"ref_{n}"]} /* {n} */,' for n in uitest_sources)
uitests_sources_phase_files = "\n".join(f'\t\t\t\t{ids[f"build_{n}"]} /* {n} in Sources */,' for n in uitest_sources)

pbxproj = f"""// !$*UTF8*$!
{{
\tarchiveVersion = 1;
\tclasses = {{
\t}};
\tobjectVersion = 56;
\tobjects = {{

/* Begin PBXBuildFile section */
{build_files_sources}
\t\t{ids['bit_ffi_swift_build']} /* bit_ffi.swift in Sources */ = {{isa = PBXBuildFile; fileRef = {ids['bit_ffi_swift_ref']} /* bit_ffi.swift */; }};
\t\t{ids['xcframework_build']} /* BitFFI.xcframework in Frameworks */ = {{isa = PBXBuildFile; fileRef = {ids['xcframework_ref']} /* BitFFI.xcframework */; }};
\t\t{ids['content_build']} /* content in Resources */ = {{isa = PBXBuildFile; fileRef = {ids['content_ref']} /* content */; }};
\t\t{ids['assets_build']} /* Assets.xcassets in Resources */ = {{isa = PBXBuildFile; fileRef = {ids['assets_ref']} /* Assets.xcassets */; }};
{uitests_build_files_sources}
/* End PBXBuildFile section */

/* Begin PBXContainerItemProxy section */
\t\t{ids['uitests_container_proxy']} /* PBXContainerItemProxy */ = {{
\t\t\tisa = PBXContainerItemProxy;
\t\t\tcontainerPortal = {ids['project']} /* Project object */;
\t\t\tproxyType = 1;
\t\t\tremoteGlobalIDString = {ids['app_target']};
\t\t\tremoteInfo = BitApp;
\t\t}};
/* End PBXContainerItemProxy section */

/* Begin PBXFileReference section */
{file_refs}
{uitests_file_refs}
\t\t{ids['bit_ffi_swift_ref']} /* bit_ffi.swift */ = {{isa = PBXFileReference; lastKnownFileType = sourcecode.swift; name = bit_ffi.swift; path = ../Generated/bit_ffi.swift; sourceTree = "<group>"; }};
\t\t{ids['xcframework_ref']} /* BitFFI.xcframework */ = {{isa = PBXFileReference; lastKnownFileType = wrapper.xcframework; name = BitFFI.xcframework; path = ../BitFFI.xcframework; sourceTree = "<group>"; }};
\t\t{ids['content_ref']} /* content */ = {{isa = PBXFileReference; lastKnownFileType = folder; name = content; path = ../../content; sourceTree = "<group>"; }};
\t\t{ids['assets_ref']} /* Assets.xcassets */ = {{isa = PBXFileReference; lastKnownFileType = folder.assetcatalog; path = Assets.xcassets; sourceTree = "<group>"; }};
\t\t{ids['app_product']} /* BitApp.app */ = {{isa = PBXFileReference; explicitFileType = wrapper.application; includeInIndex = 0; path = BitApp.app; sourceTree = BUILT_PRODUCTS_DIR; }};
\t\t{ids['uitests_product']} /* BitAppUITests.xctest */ = {{isa = PBXFileReference; explicitFileType = wrapper.cfbundle; includeInIndex = 0; path = BitAppUITests.xctest; sourceTree = BUILT_PRODUCTS_DIR; }};
/* End PBXFileReference section */

/* Begin PBXFrameworksBuildPhase section */
\t\t{ids['frameworks_phase']} /* Frameworks */ = {{
\t\t\tisa = PBXFrameworksBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
\t\t\t\t{ids['xcframework_build']} /* BitFFI.xcframework in Frameworks */,
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t}};
\t\t{ids['uitests_frameworks_phase']} /* Frameworks */ = {{
\t\t\tisa = PBXFrameworksBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t}};
/* End PBXFrameworksBuildPhase section */

/* Begin PBXGroup section */
\t\t{ids['main_group']} = {{
\t\t\tisa = PBXGroup;
\t\t\tchildren = (
\t\t\t\t{ids['app_group']} /* BitApp */,
\t\t\t\t{ids['uitests_group']} /* BitAppUITests */,
\t\t\t\t{ids['generated_group']} /* Generated */,
\t\t\t\t{ids['xcframework_ref']} /* BitFFI.xcframework */,
\t\t\t\t{ids['content_ref']} /* content */,
\t\t\t\t{ids['products_group']} /* Products */,
\t\t\t);
\t\t\tsourceTree = "<group>";
\t\t}};
\t\t{ids['app_group']} /* BitApp */ = {{
\t\t\tisa = PBXGroup;
\t\t\tchildren = (
{group_children}
\t\t\t\t{ids['assets_ref']} /* Assets.xcassets */,
\t\t\t);
\t\t\tpath = BitApp;
\t\t\tsourceTree = "<group>";
\t\t}};
\t\t{ids['uitests_group']} /* BitAppUITests */ = {{
\t\t\tisa = PBXGroup;
\t\t\tchildren = (
{uitests_group_children}
\t\t\t);
\t\t\tpath = BitAppUITests;
\t\t\tsourceTree = "<group>";
\t\t}};
\t\t{ids['generated_group']} /* Generated */ = {{
\t\t\tisa = PBXGroup;
\t\t\tchildren = (
\t\t\t\t{ids['bit_ffi_swift_ref']} /* bit_ffi.swift */,
\t\t\t);
\t\t\tname = Generated;
\t\t\tsourceTree = "<group>";
\t\t}};
\t\t{ids['products_group']} /* Products */ = {{
\t\t\tisa = PBXGroup;
\t\t\tchildren = (
\t\t\t\t{ids['app_product']} /* BitApp.app */,
\t\t\t\t{ids['uitests_product']} /* BitAppUITests.xctest */,
\t\t\t);
\t\t\tname = Products;
\t\t\tsourceTree = "<group>";
\t\t}};
/* End PBXGroup section */

/* Begin PBXNativeTarget section */
\t\t{ids['app_target']} /* BitApp */ = {{
\t\t\tisa = PBXNativeTarget;
\t\t\tbuildConfigurationList = {ids['target_config_list']} /* Build configuration list for PBXNativeTarget "BitApp" */;
\t\t\tbuildPhases = (
\t\t\t\t{ids['sources_phase']} /* Sources */,
\t\t\t\t{ids['frameworks_phase']} /* Frameworks */,
\t\t\t\t{ids['resources_phase']} /* Resources */,
\t\t\t);
\t\t\tbuildRules = (
\t\t\t);
\t\t\tdependencies = (
\t\t\t);
\t\t\tname = BitApp;
\t\t\tproductName = BitApp;
\t\t\tproductReference = {ids['app_product']} /* BitApp.app */;
\t\t\tproductType = "com.apple.product-type.application";
\t\t}};
\t\t{ids['uitests_target']} /* BitAppUITests */ = {{
\t\t\tisa = PBXNativeTarget;
\t\t\tbuildConfigurationList = {ids['uitests_config_list']} /* Build configuration list for PBXNativeTarget "BitAppUITests" */;
\t\t\tbuildPhases = (
\t\t\t\t{ids['uitests_sources_phase']} /* Sources */,
\t\t\t\t{ids['uitests_frameworks_phase']} /* Frameworks */,
\t\t\t\t{ids['uitests_resources_phase']} /* Resources */,
\t\t\t);
\t\t\tbuildRules = (
\t\t\t);
\t\t\tdependencies = (
\t\t\t\t{ids['uitests_dependency']} /* PBXTargetDependency */,
\t\t\t);
\t\t\tname = BitAppUITests;
\t\t\tproductName = BitAppUITests;
\t\t\tproductReference = {ids['uitests_product']} /* BitAppUITests.xctest */;
\t\t\tproductType = "com.apple.product-type.bundle.ui-testing";
\t\t}};
/* End PBXNativeTarget section */

/* Begin PBXProject section */
\t\t{ids['project']} /* Project object */ = {{
\t\t\tisa = PBXProject;
\t\t\tattributes = {{
\t\t\t\tBuildIndependentTargetsInParallel = 1;
\t\t\t\tLastSwiftUpdateCheck = 1620;
\t\t\t\tLastUpgradeCheck = 1620;
\t\t\t\tTargetAttributes = {{
\t\t\t\t\t{ids['uitests_target']} = {{
\t\t\t\t\t\tTestTargetID = {ids['app_target']};
\t\t\t\t\t}};
\t\t\t\t}};
\t\t\t}};
\t\t\tbuildConfigurationList = {ids['proj_config_list']} /* Build configuration list for PBXProject "BitApp" */;
\t\t\tcompatibilityVersion = "Xcode 14.0";
\t\t\tdevelopmentRegion = en;
\t\t\thasScannedForEncodings = 0;
\t\t\tknownRegions = (
\t\t\t\ten,
\t\t\t\tBase,
\t\t\t);
\t\t\tmainGroup = {ids['main_group']};
\t\t\tproductRefGroup = {ids['products_group']} /* Products */;
\t\t\tprojectDirPath = "";
\t\t\tprojectRoot = "";
\t\t\ttargets = (
\t\t\t\t{ids['app_target']} /* BitApp */,
\t\t\t\t{ids['uitests_target']} /* BitAppUITests */,
\t\t\t);
\t\t}};
/* End PBXProject section */

/* Begin PBXResourcesBuildPhase section */
\t\t{ids['resources_phase']} /* Resources */ = {{
\t\t\tisa = PBXResourcesBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
\t\t\t\t{ids['content_build']} /* content in Resources */,
\t\t\t\t{ids['assets_build']} /* Assets.xcassets in Resources */,
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t}};
\t\t{ids['uitests_resources_phase']} /* Resources */ = {{
\t\t\tisa = PBXResourcesBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t}};
/* End PBXResourcesBuildPhase section */

/* Begin PBXSourcesBuildPhase section */
\t\t{ids['sources_phase']} /* Sources */ = {{
\t\t\tisa = PBXSourcesBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
{sources_phase_files}
\t\t\t\t{ids['bit_ffi_swift_build']} /* bit_ffi.swift in Sources */,
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t}};
\t\t{ids['uitests_sources_phase']} /* Sources */ = {{
\t\t\tisa = PBXSourcesBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
{uitests_sources_phase_files}
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t}};
/* End PBXSourcesBuildPhase section */

/* Begin PBXTargetDependency section */
\t\t{ids['uitests_dependency']} /* PBXTargetDependency */ = {{
\t\t\tisa = PBXTargetDependency;
\t\t\ttarget = {ids['app_target']} /* BitApp */;
\t\t\ttargetProxy = {ids['uitests_container_proxy']} /* PBXContainerItemProxy */;
\t\t}};
/* End PBXTargetDependency section */

/* Begin XCBuildConfiguration section */
\t\t{ids['proj_debug']} /* Debug */ = {{
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {{
\t\t\t\tALWAYS_SEARCH_USER_PATHS = NO;
\t\t\t\tCLANG_ENABLE_MODULES = YES;
\t\t\t\tCLANG_ENABLE_OBJC_ARC = YES;
\t\t\t\tCODE_SIGNING_ALLOWED = NO;
\t\t\t\tCODE_SIGNING_REQUIRED = NO;
\t\t\t\tCOPY_PHASE_STRIP = NO;
\t\t\t\tDEBUG_INFORMATION_FORMAT = dwarf;
\t\t\t\tENABLE_TESTABILITY = YES;
\t\t\t\tGCC_OPTIMIZATION_LEVEL = 0;
\t\t\t\tGCC_PREPROCESSOR_DEFINITIONS = (
\t\t\t\t\t"DEBUG=1",
\t\t\t\t\t"$(inherited)",
\t\t\t\t);
\t\t\t\tIPHONEOS_DEPLOYMENT_TARGET = 17.0;
\t\t\t\tMTL_ENABLE_DEBUG_INFO = YES;
\t\t\t\tONLY_ACTIVE_ARCH = YES;
\t\t\t\tSDKROOT = iphoneos;
\t\t\t\tSWIFT_ACTIVE_COMPILATION_CONDITIONS = DEBUG;
\t\t\t\tSWIFT_OPTIMIZATION_LEVEL = "-Onone";
\t\t\t\tSWIFT_VERSION = 5.0;
\t\t\t\tTARGETED_DEVICE_FAMILY = 1;
\t\t\t}};
\t\t\tname = Debug;
\t\t}};
\t\t{ids['proj_release']} /* Release */ = {{
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {{
\t\t\t\tALWAYS_SEARCH_USER_PATHS = NO;
\t\t\t\tCLANG_ENABLE_MODULES = YES;
\t\t\t\tCLANG_ENABLE_OBJC_ARC = YES;
\t\t\t\tCODE_SIGNING_ALLOWED = NO;
\t\t\t\tCODE_SIGNING_REQUIRED = NO;
\t\t\t\tCOPY_PHASE_STRIP = NO;
\t\t\t\tDEBUG_INFORMATION_FORMAT = "dwarf-with-dsym";
\t\t\t\tGCC_OPTIMIZATION_LEVEL = s;
\t\t\t\tIPHONEOS_DEPLOYMENT_TARGET = 17.0;
\t\t\t\tMTL_ENABLE_DEBUG_INFO = NO;
\t\t\t\tSDKROOT = iphoneos;
\t\t\t\tSWIFT_COMPILATION_MODE = wholemodule;
\t\t\t\tSWIFT_OPTIMIZATION_LEVEL = "-O";
\t\t\t\tSWIFT_VERSION = 5.0;
\t\t\t\tTARGETED_DEVICE_FAMILY = 1;
\t\t\t\tVALIDATE_PRODUCT = YES;
\t\t\t}};
\t\t\tname = Release;
\t\t}};
\t\t{ids['target_debug']} /* Debug */ = {{
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {{
\t\t\t\tASSETCATALOG_COMPILER_APPICON_NAME = AppIcon;
\t\t\t\tINFOPLIST_KEY_CFBundleDisplayName = BIT;
\t\t\t\tASSETCATALOG_COMPILER_GENERATE_ASSET_SYMBOLS = NO;
\t\t\t\tCODE_SIGN_STYLE = Automatic;
\t\t\t\tCODE_SIGNING_ALLOWED = NO;
\t\t\t\tCODE_SIGNING_REQUIRED = NO;
\t\t\t\tENABLE_PREVIEWS = YES;
\t\t\t\tGENERATE_INFOPLIST_FILE = YES;
\t\t\t\tINFOPLIST_KEY_UIApplicationSupportsIndirectInputEvents = YES;
\t\t\t\tINFOPLIST_KEY_UILaunchScreen_Generation = YES;
\t\t\t\tINFOPLIST_KEY_UISupportedInterfaceOrientations = UIInterfaceOrientationPortrait;
\t\t\t\tLD_RUNPATH_SEARCH_PATHS = (
\t\t\t\t\t"$(inherited)",
\t\t\t\t\t"@executable_path/Frameworks",
\t\t\t\t);
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = dev.bit.app;
\t\t\t\tPRODUCT_NAME = "$(TARGET_NAME)";
\t\t\t\tSWIFT_EMIT_LOC_STRINGS = YES;
\t\t\t\tSWIFT_VERSION = 5.0;
\t\t\t\tTARGETED_DEVICE_FAMILY = 1;
\t\t\t}};
\t\t\tname = Debug;
\t\t}};
\t\t{ids['target_release']} /* Release */ = {{
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {{
\t\t\t\tASSETCATALOG_COMPILER_APPICON_NAME = AppIcon;
\t\t\t\tINFOPLIST_KEY_CFBundleDisplayName = BIT;
\t\t\t\tASSETCATALOG_COMPILER_GENERATE_ASSET_SYMBOLS = NO;
\t\t\t\tCODE_SIGN_STYLE = Automatic;
\t\t\t\tCODE_SIGNING_ALLOWED = NO;
\t\t\t\tCODE_SIGNING_REQUIRED = NO;
\t\t\t\tENABLE_PREVIEWS = YES;
\t\t\t\tGENERATE_INFOPLIST_FILE = YES;
\t\t\t\tINFOPLIST_KEY_UIApplicationSupportsIndirectInputEvents = YES;
\t\t\t\tINFOPLIST_KEY_UILaunchScreen_Generation = YES;
\t\t\t\tINFOPLIST_KEY_UISupportedInterfaceOrientations = UIInterfaceOrientationPortrait;
\t\t\t\tLD_RUNPATH_SEARCH_PATHS = (
\t\t\t\t\t"$(inherited)",
\t\t\t\t\t"@executable_path/Frameworks",
\t\t\t\t);
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = dev.bit.app;
\t\t\t\tPRODUCT_NAME = "$(TARGET_NAME)";
\t\t\t\tSWIFT_EMIT_LOC_STRINGS = YES;
\t\t\t\tSWIFT_VERSION = 5.0;
\t\t\t\tTARGETED_DEVICE_FAMILY = 1;
\t\t\t}};
\t\t\tname = Release;
\t\t}};
\t\t{ids['uitests_debug']} /* Debug */ = {{
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {{
\t\t\t\tCODE_SIGN_STYLE = Automatic;
\t\t\t\tCODE_SIGNING_ALLOWED = NO;
\t\t\t\tCODE_SIGNING_REQUIRED = NO;
\t\t\t\tGENERATE_INFOPLIST_FILE = YES;
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = "dev.bit.app.UITests";
\t\t\t\tPRODUCT_NAME = "$(TARGET_NAME)";
\t\t\t\tSWIFT_EMIT_LOC_STRINGS = NO;
\t\t\t\tSWIFT_VERSION = 5.0;
\t\t\t\tTARGETED_DEVICE_FAMILY = 1;
\t\t\t\tTEST_TARGET_NAME = BitApp;
\t\t\t}};
\t\t\tname = Debug;
\t\t}};
\t\t{ids['uitests_release']} /* Release */ = {{
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {{
\t\t\t\tCODE_SIGN_STYLE = Automatic;
\t\t\t\tCODE_SIGNING_ALLOWED = NO;
\t\t\t\tCODE_SIGNING_REQUIRED = NO;
\t\t\t\tGENERATE_INFOPLIST_FILE = YES;
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = "dev.bit.app.UITests";
\t\t\t\tPRODUCT_NAME = "$(TARGET_NAME)";
\t\t\t\tSWIFT_EMIT_LOC_STRINGS = NO;
\t\t\t\tSWIFT_VERSION = 5.0;
\t\t\t\tTARGETED_DEVICE_FAMILY = 1;
\t\t\t\tTEST_TARGET_NAME = BitApp;
\t\t\t}};
\t\t\tname = Release;
\t\t}};
/* End XCBuildConfiguration section */

/* Begin XCConfigurationList section */
\t\t{ids['proj_config_list']} /* Build configuration list for PBXProject "BitApp" */ = {{
\t\t\tisa = XCConfigurationList;
\t\t\tbuildConfigurations = (
\t\t\t\t{ids['proj_debug']} /* Debug */,
\t\t\t\t{ids['proj_release']} /* Release */,
\t\t\t);
\t\t\tdefaultConfigurationIsVisible = 0;
\t\t\tdefaultConfigurationName = Release;
\t\t}};
\t\t{ids['target_config_list']} /* Build configuration list for PBXNativeTarget "BitApp" */ = {{
\t\t\tisa = XCConfigurationList;
\t\t\tbuildConfigurations = (
\t\t\t\t{ids['target_debug']} /* Debug */,
\t\t\t\t{ids['target_release']} /* Release */,
\t\t\t);
\t\t\tdefaultConfigurationIsVisible = 0;
\t\t\tdefaultConfigurationName = Release;
\t\t}};
\t\t{ids['uitests_config_list']} /* Build configuration list for PBXNativeTarget "BitAppUITests" */ = {{
\t\t\tisa = XCConfigurationList;
\t\t\tbuildConfigurations = (
\t\t\t\t{ids['uitests_debug']} /* Debug */,
\t\t\t\t{ids['uitests_release']} /* Release */,
\t\t\t);
\t\t\tdefaultConfigurationIsVisible = 0;
\t\t\tdefaultConfigurationName = Release;
\t\t}};
/* End XCConfigurationList section */
\t}};
\trootObject = {ids['project']} /* Project object */;
}}
"""

out_dir = os.path.join(PROJECT_DIR, "BitApp.xcodeproj")
os.makedirs(out_dir, exist_ok=True)
with open(os.path.join(out_dir, "project.pbxproj"), "w") as f:
    f.write(pbxproj)

print(f"Wrote {out_dir}/project.pbxproj")

# A shared scheme is required for `xcodebuild test` to see BitAppUITests
# as a testable — without one, xcodebuild only auto-vends a build-only
# scheme for the app target. Generated here (not by Xcode's GUI) so the
# BlueprintIdentifiers below stay in sync with the pbxproj IDs above.
xcscheme = f"""<?xml version="1.0" encoding="UTF-8"?>
<Scheme
   LastUpgradeVersion = "1620"
   version = "1.7">
   <BuildAction
      parallelizeBuildables = "YES"
      buildImplicitDependencies = "YES">
      <BuildActionEntries>
         <BuildActionEntry
            buildForTesting = "YES"
            buildForRunning = "YES"
            buildForProfiling = "YES"
            buildForArchiving = "YES"
            buildForAnalyzing = "YES">
            <BuildableReference
               BuildableIdentifier = "primary"
               BlueprintIdentifier = "{ids['app_target']}"
               BuildableName = "BitApp.app"
               BlueprintName = "BitApp"
               ReferencedContainer = "container:BitApp.xcodeproj">
            </BuildableReference>
         </BuildActionEntry>
         <BuildActionEntry
            buildForTesting = "YES"
            buildForRunning = "NO"
            buildForProfiling = "NO"
            buildForArchiving = "NO"
            buildForAnalyzing = "NO">
            <BuildableReference
               BuildableIdentifier = "primary"
               BlueprintIdentifier = "{ids['uitests_target']}"
               BuildableName = "BitAppUITests.xctest"
               BlueprintName = "BitAppUITests"
               ReferencedContainer = "container:BitApp.xcodeproj">
            </BuildableReference>
         </BuildActionEntry>
      </BuildActionEntries>
   </BuildAction>
   <TestAction
      buildConfiguration = "Debug"
      selectedDebuggerIdentifier = "Xcode.DebuggerFoundation.Debugger.LLDB"
      selectedLauncherIdentifier = "Xcode.DebuggerFoundation.Launcher.LLDB"
      shouldUseLaunchSchemeArgsEnv = "YES">
      <Testables>
         <TestableReference
            skipped = "NO">
            <BuildableReference
               BuildableIdentifier = "primary"
               BlueprintIdentifier = "{ids['uitests_target']}"
               BuildableName = "BitAppUITests.xctest"
               BlueprintName = "BitAppUITests"
               ReferencedContainer = "container:BitApp.xcodeproj">
            </BuildableReference>
         </TestableReference>
      </Testables>
   </TestAction>
   <LaunchAction
      buildConfiguration = "Debug"
      selectedDebuggerIdentifier = "Xcode.DebuggerFoundation.Debugger.LLDB"
      selectedLauncherIdentifier = "Xcode.DebuggerFoundation.Launcher.LLDB"
      launchStyle = "0"
      useCustomWorkingDirectory = "NO"
      ignoresPersistentStateOnLaunch = "NO"
      debugDocumentVersioning = "YES"
      debugServiceExtension = "internal"
      allowLocationSimulation = "YES">
      <BuildableProductRunnable
         runnableDebuggingMode = "0">
         <BuildableReference
            BuildableIdentifier = "primary"
            BlueprintIdentifier = "{ids['app_target']}"
            BuildableName = "BitApp.app"
            BlueprintName = "BitApp"
            ReferencedContainer = "container:BitApp.xcodeproj">
         </BuildableReference>
      </BuildableProductRunnable>
   </LaunchAction>
</Scheme>
"""

scheme_dir = os.path.join(out_dir, "xcshareddata", "xcschemes")
os.makedirs(scheme_dir, exist_ok=True)
with open(os.path.join(scheme_dir, "BitApp.xcscheme"), "w") as f:
    f.write(xcscheme)

print(f"Wrote {scheme_dir}/BitApp.xcscheme")
