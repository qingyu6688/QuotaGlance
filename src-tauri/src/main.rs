// 发布构建在 Windows 上不显示额外控制台窗口。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    quota_glance_lib::run()
}
