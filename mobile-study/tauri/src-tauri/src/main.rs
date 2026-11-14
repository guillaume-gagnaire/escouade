// Le binaire de bureau (inutile à l'étude, mais Tauri le veut) : la maquette tourne sur le téléphone.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    escouade_mobile_tauri_lib::run()
}
