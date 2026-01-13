# WebSocket Ping-Pong Benchmark in Rust

Полная переписка C-бенчмарка на Rust с поддержкой TLS, async I/O и статистики производительности.

## Возможности

- ✅ WebSocket клиент с поддержкой TLS (native-tls)
- ✅ Асинхронные бенчмарки на базе tokio
- ✅ TCP и UDP бенчмарки
- ✅ Multi-connection тесты (50 клиентов)
- ✅ Измерение CPU time (user, system, wall)
- ✅ Статистика памяти
- ✅ RTT статистика (avg, median, min, max)
- ✅ Интерактивное консольное меню
- ✅ Unit тесты для всех основных модулей

## Сборка

```bash
cargo build --release
```

Исполняемый файл будет в `target/release/websocket_benchmark`

## Использование

### Интерактивный режим

```bash
./bin/websocket_benchmark
```

### Command-line режим

```bash
# Запустить бенчмарк 1
./bin/websocket_benchmark --benchmark 1

# С заданным хостом и портом
./bin/websocket_benchmark -b 1 -h 192.168.1.100 -p 8443

# С заданным количеством итераций
./bin/websocket_benchmark -b 1 -c 100

# Тихий режим
./bin/websocket_benchmark -b 1 -q
```

## Доступные бенчмарки

1. **async + Native TLS** - Асинхронный бенчмарк с TLS
2. **sync + Native TLS** - Синхронный бенчмарк с TLS (blocking I/O)
5. **Run ALL TLS benchmarks** - Запустить все TLS бенчмарки
6. **Multi-Connection** - Многоконнекционный тест (50 клиентов)
7. **TCP benchmark** - TCP бенчмарк без TLS
8. **UDP benchmark** - UDP бенчмарк

## Зависимости

- `tokio` - Async runtime
- `tokio-tungstenite` - WebSocket клиент
- `native-tls` - TLS поддержка
- `base64` - Base64 кодирование
- `clap` - CLI парсер
- `anyhow` - Обработка ошибок

## Запуск тестов

```bash
cargo test
```

## Структура проекта

```
new_rust/
├── Cargo.toml          # Зависимости проекта
├── src/
│   ├── main.rs         # Точка входа
│   ├── benchmark.rs    # Бенчмарки
│   ├── cli.rs          # CLI интерфейс
│   ├── stats.rs        # Статистика
│   ├── utils.rs        # Утилиты
│   └── websocket.rs    # WebSocket фреймы
├── bin/
│   └── websocket_benchmark  # Исполняемый файл (linux64)
└── README.md
```

## Результаты тестов

```
running 11 tests
test benchmark::tests::test_benchmark_config_default ... ok
test stats::tests::test_cpu_time_creation ... ok
test stats::tests::test_rtt_stats_calculation ... ok
test stats::tests::test_rtt_stats_empty ... ok
test stats::tests::test_rtt_stats_even_count ... ok
test utils::tests::test_base64_roundtrip ... ok
test utils::tests::test_websocket_key_generation ... ok
test websocket::tests::test_create_text_frame ... ok
test websocket::tests::test_large_frame ... ok
test websocket::tests::test_parse_frame ... ok
test websocket::tests::test_ping_frame ... ok

test result: ok. 11 passed; 0 failed
```

## Сравнение с C версией

| Функционал | C версия | Rust версия |
|-----------|-----------|-------------|
| WebSocket TLS | ✅ | ✅ |
| kTLS | ✅ | ⚠️ (Linux only, требует OpenSSL 3.0+) |
| epoll | ✅ | ⚠️ (через tokio) |
| io_uring | ✅ | ❌ (пока не реализовано) |
| Multi-connection | ✅ | ✅ |
| CPU/Stats | ✅ | ✅ |
| TCP/UDP | ✅ | ✅ |

## Лицензия

MIT

## Авторы

Benchmark Team
