# Ebike Power Meter
Индикатор потребляемой мощности с индикацией для контроллеров асинхронных двигателей, поддерживающих дисплей [LCD3](https://github.com/stancecoke/BMSBattery_S_controllers_firmware/tree/Master/documentation)

## Сборка
В WSL2
1. [espup](https://github.com/esp-rs/espup), Установить тулчеин для `ESP32`
2. Установить компилятор `GCC` для `ESP8266`
3. Установить на хосте `cargo install espflash@=1.5.0`
4. Скопировать `.env.example` -> `.env` и вписать правильный порт `SERIAL_PORT` на __хосте__.
4. `cargo run` - Сборка и прошивка.