# NRS — техническая спецификация этапа 1

**Документ:** NRS-TS-001  
**Версия:** 1.0  
**Статус:** проектная спецификация, не описание уже реализованной системы  
**Целевая платформа разработки:** Windows / Linux / macOS  
**Язык:** Rust, edition 2024  
**Формат проекта:** \`.nrsproj\` (полная спецификация сериализации — отдельный документ)

---

## 1. Назначение и границы этапа

Первый этап создаёт проверяемое ядро NRS: загрузку и разбор NES ROM, детерминированную модель аппаратного исполнения, цикловой CPU Ricoh 2A03, минимально необходимую модель PPU/APU и памяти, сбор машинных трасс, первичный статический анализ и lossless IR, пригодный для последующего Blueprint-редактора.

Цель этапа — не сделать сразу весь продукт с редактором уровней, визуальными графами, Timeline и компилятором. Цель — создать **достоверный источник машинного поведения**, на котором эти подсистемы смогут строиться без подмены реальной семантики приблизительными игровыми сценариями.

### 1.1. Результат этапа

К концу этапа должен существовать headless runtime, который:

1. принимает валидный iNES ROM и поддерживаемый поднабор NES 2.0;
2. строит объект картриджа с явной информацией о PRG/CHR, mapper, mirroring и регионе;
3. выполняет CPU 2A03 по тактам, а не атомарно по инструкциям;
4. маршрутизирует обращения через CPU/PPU bus и mapper;
5. поддерживает базовый аппаратный цикл PPU, vblank/NMI и получение кадра;
6. моделирует регистры и базовые таймеры APU с детерминированным продвижением времени;
7. записывает трассу обращений к шине и исполнения инструкций;
8. выполняет начальный статический анализ в контексте банков и векторов прерываний;
9. формирует IR, сохраняющий исходные байты, адресацию, opcode, машинную семантику, происхождение и степень достоверности;
10. позволяет остановить исполнение, получить snapshot и восстановить его с тем же последующим результатом.

### 1.2. Что не входит в первый релиз

| Подсистема | В первом этапе | Отложено |
|---|---|---|
| CPU | Цикловое исполнение 2A03; все байты opcode декодируются; официальные и стабильные неофициальные инструкции исполняются по спецификации; отдельная политика для нестабильных opcode | Аппаратно-эквивалентная модель аналоговых/нестабильных эффектов на всех ревизиях чипа |
| PPU | Регистры, адресное пространство, тайминг scanline/dot, vblank, NMI, базовый framebuffer и необходимая для диагностических ROM модель рендеринга | Полная проверенная точность всех rendering edge cases, light gun, расширения отдельных mapper |
| APU | Регистры, frame sequencer, базовые каналы и детерминированное продвижение таймеров; интерфейс аудиосэмплов | Высокоточная аналоговая модель микшера, фильтры конкретных ревизий, аудиопериферия |
| Mapper | Архитектура расширяемых mapper; NROM как обязательный вертикальный срез; следующий приоритет — UxROM/CNROM | Большая библиотека mapper, нестандартные платы, расширенные аудиоканалы |
| ROM analysis | Разбор ROM, reset/vector seeds, bank-aware статический проход, трассы динамического исполнения, начальные CFG/IR | Полное автоматическое разделение кода и данных, гарантированная реконструкция исходной логики |
| Blueprint | Стабильная модель узлов и связей IR, пригодная для отображения | Полноценный интерактивный редактор графов и библиотека высокоуровневых игровых нод |
| Timeline | Контракт snapshot/event log и тест восстановления | UI таймлайна, перемотка в реальном времени, ветвление истории |
| Split Screen | Контракт двух независимых экземпляров runtime и синхронного ввода | UI сравнения оригинала и реконструкции |
| ROM build | Контракт round-trip и сохранение неизменённых байтов | Полный редактор машинного кода, relocations, оптимизация и компоновка изменённых банков |
| Export | Архитектурная граница runtime package | Standalone export для desktop/mobile |

**Ограничение:** статический анализ не может безошибочно определить, какие байты ROM являются кодом, таблицами или графикой. Неизвестные диапазоны должны оставаться неизвестными, а не автоматически превращаться в выдуманные инструкции.

---

## 2. Архитектурные принципы

### 2.1. Blueprint-first без потери машинной семантики

Blueprint в NRS — не замена эмулятору и не набор предположительных игровых действий. Это представление подтверждённых машинных операций и связей между ними.

Каждый узел исполняемого графа должен иметь один из типов происхождения:

- **Decoded** — байты декодированы известной таблицей opcode;
- **Observed** — поведение подтверждено динамической трассой;
- **Inferred** — связь или назначение выведены анализатором;
- **UserDefined** — добавлено пользователем;
- **Opaque** — машинный фрагмент сохранён как непрозрачный блок без утверждения о его смысле.

Имя вроде \`UpdatePlayer\` может быть пользовательской гипотезой, но не должно записываться как восстановленное исходное имя, если его нет в символах/отладочных данных.

### 2.2. Источник истины

| Данные | Источник истины |
|---|---|
| Байты ROM, заголовок, хеш | \`nrs-rom\` |
| Состояние регистров и микрошаг CPU | \`nrs-cpu\` |
| Состояние PPU/APU | соответствующие crates |
| Маршрутизация и побочные эффекты памяти | \`nrs-memory\` + mapper |
| Порядок тактов и согласованность устройств | \`nrs-emulator\` |
| Факты наблюдения | trace/event stream |
| Декодирование и гипотезы | \`nrs-analysis\` |
| Машинное и визуальное представление | \`nrs-ir\` |
| Пользовательские правки и проектные метаданные | \`nrs-project\` |

Анализатор не меняет аппаратное состояние задним числом. Blueprint-редактор не должен подменять исходные байты без явной операции компиляции.

### 2.3. Детерминизм

При одинаковых ROM, версии runtime, конфигурации региона и последовательности входных событий исполнение должно давать одинаковые:

- последовательность CPU-visible bus accesses;
- состояния CPU/PPU/APU на контрольных точках;
- кадры framebuffer;
- контрольные хеши snapshot.

Плавающая точка не используется для координации аппаратных часов. Время устройств задаётся целочисленными счётчиками и рациональными делителями.

---

## 3. Структура Rust workspace

Предлагаемая целевая структура:

\`\`\`text
NRS/
├── Cargo.toml
├── README.md
├── crates/
│   ├── nrs-core/
│   ├── nrs-rom/
│   ├── nrs-cpu/
│   ├── nrs-mappers/
│   ├── nrs-memory/
│   ├── nrs-ppu/
│   ├── nrs-apu/
│   ├── nrs-ir/
│   ├── nrs-emulator/
│   ├── nrs-analysis/
│   └── nrs-project/
├── apps/
│   └── nrs-cli/
├── docs/
└── tests/
    ├── fixtures/
    ├── roms/
    └── expected/
\`\`\`

На момент написания документа в репозитории создан workspace только с \`nrs-core\` и \`nrs-cpu\`. Остальные crates — планируемая структура. Добавлять их следует постепенно, когда появляется реальная ответственность и тесты; не создавать пустые crates только ради схемы.

### 3.1. Ответственность crates

| Crate | Ответственность | Не должен делать |
|---|---|---|
| \`nrs-core\` | Общие типы адресов/тактов, Region, ошибки, ID, рациональный clock divider, версии snapshot, общие trace-типы | Знать конкретные opcode, mapper и UI |
| \`nrs-rom\` | Парсинг iNES/NES 2.0, валидация заголовка, trainer, PRG/CHR ROM/RAM, метаданные и хеш исходного образа | Исполнять код или менять ROM |
| \`nrs-cpu\` | Ricoh 2A03: регистры, opcode table, микропоследовательности, IRQ/NMI/RESET, CPU-visible bus contract | Самостоятельно адресовать RAM/mapper без bus |
| \`nrs-mappers\` | Контракт mapper и реализации банкового переключения, mirroring, IRQ/CHR mapping | Знать внутренности CPU или UI |
| \`nrs-memory\` | CPU bus, PPU bus, RAM, open bus, зеркалирование регистров, маршрутизация к устройствам и cartridge | Декодировать инструкции |
| \`nrs-ppu\` | Регистры PPU, VRAM/OAM/palette, тайминг, NMI, рендеринг | Читать ROM напрямую в обход PPU bus |
| \`nrs-apu\` | Регистры APU, каналы, frame sequencer, DMC request/IRQ, audio sample stream | Управлять CPU циклом самостоятельно |
| \`nrs-ir\` | Схема IR: ROM map, instructions, blocks, edges, provenance, confidence, annotations | Решать, является ли гипотеза доказанным фактом |
| \`nrs-emulator\` | Сборка устройств, master scheduler, ввод, DMA arbitration, run loop, snapshots, trace sink | Содержать UI или эвристики анализа |
| \`nrs-analysis\` | Статический decode, CFG, bank-aware analysis, trace correlation, гипотезы функций/данных | Менять машинную семантику |
| \`nrs-project\` | Проектные метаданные, сериализация IR, ссылки на ROM, настройки анализа, версии формата | Быть источником состояния работающего эмулятора |
| \`nrs-cli\` | Headless команды: inspect ROM, run test, trace, analyze, snapshot diagnostics | Дублировать логику crates |

### 3.2. Направление зависимостей

\`\`\`text
                 nrs-cli
                    |
               nrs-project
                    |
      +-------------+-------------+
      |                           |
  nrs-analysis                 nrs-emulator
      |                           |
    nrs-ir             +----------+----------+
      |                |     |     |         |
      +---- nrs-core --+   nrs-cpu nrs-ppu nrs-apu
                           |       |     |
                       nrs-memory-+-----+
                           |
                       nrs-mappers
                           |
                         nrs-rom
\`\`\`

Схема показывает логические связи, а не разрешение на циклические зависимости. Реальная компоновка должна оставаться ациклической:

- \`nrs-core\` не зависит от остальных crates;
- \`nrs-rom\` зависит только от \`nrs-core\`;
- \`nrs-cpu\`, \`nrs-ppu\`, \`nrs-apu\` зависят от общих контрактов, но не друг от друга;
- \`nrs-mappers\` использует модель картриджа и общие bus-типы;
- \`nrs-memory\` соединяет bus-контракты устройств и mapper;
- \`nrs-emulator\` является composition root и связывает устройства;
- \`nrs-analysis\` потребляет ROM metadata, opcode metadata и trace stream;
- \`nrs-ir\` не зависит от runtime и может сохраняться независимо.

Если для связи двух crates возникает цикл, общий контракт выносится в нижележащий crate, а не решается взаимной зависимостью.

---

## 4. Публичные API и контракты

Публичные API ниже — целевые сигнатуры и смысловые контракты. Это не утверждение, что они уже реализованы. Детали типов могут уточняться в процессе кодирования, но семантика должна сохраняться.

### 4.1. Общие типы — \`nrs-core\`

\`\`\`rust
pub enum Region {
    Ntsc,
    Pal,
    Dendy,
}

pub struct MasterTick(pub u64);
pub struct CpuCycle(pub u64);
pub struct PpuDot(pub u64);

pub struct ClockRatio {
    pub numerator: u32,
    pub denominator: u32,
}

pub trait SnapshotComponent {
    type Snapshot;

    fn capture(&self) -> Self::Snapshot;
    fn restore(&mut self, snapshot: &Self::Snapshot)
        -> Result<(), SnapshotError>;
}
\`\`\`

Контракты:

- \`MasterTick\`, \`CpuCycle\`, \`PpuDot\` не смешиваются в одном неразмеченном \`u64\`;
- clock divider хранит фазу и при восстановлении продолжает ту же последовательность делений;
- ошибки имеют машинно-читаемые варианты (invalid header, unsupported mapper, truncated ROM, incompatible snapshot и т. п.);
- версии snapshot и project format независимы друг от друга.

### 4.2. ROM — \`nrs-rom\`

\`\`\`rust
pub struct RomImage {
    pub format: RomFormat,
    pub header: RomHeader,
    pub trainer: Option<Box<[u8]>>,
    pub prg_rom: Box<[u8]>,
    pub chr_rom: Option<Box<[u8]>>,
    pub chr_ram_size: usize,
    pub source_hash: [u8; 32],
}

pub fn parse_rom(bytes: &[u8]) -> Result<RomImage, RomError>;
\`\`\`

Обязательные проверки:

- сигнатура \`NES\\x1A\`;
- версия iNES/NES 2.0 по правилам формата;
- переполнение размеров и выход за пределы входного буфера;
- trainer учитывается при расчёте смещений;
- PRG ROM не может молча обрезаться;
- отсутствие CHR ROM трактуется как CHR RAM только при корректных метаданных/правиле формата;
- неизвестный mapper сохраняется в metadata, но запуск запрещается с явной ошибкой, если реализации нет.

Парсер не должен «исправлять» повреждённые заголовки без сохранения исходных байтов и диагностического сообщения.

### 4.3. CPU — \`nrs-cpu\`

\`\`\`rust
pub struct OpcodeDesc {
    pub opcode: u8,
    pub mnemonic: Mnemonic,
    pub operation: Operation,
    pub addressing: AddressingMode,
    pub instruction_class: InstructionClass,
    pub length: u8,
    pub nominal_cycles: u8,
    pub legality: OpcodeLegality,
}

pub fn decode_opcode(opcode: u8) -> &'static OpcodeDesc;

pub trait CpuBus {
    fn read(&mut self, address: u16, kind: CpuAccessKind) -> u8;
    fn write(&mut self, address: u16, value: u8, kind: CpuAccessKind);
}

impl Cpu {
    pub fn clock<B: CpuBus>(&mut self, bus: &mut B) -> CpuCycleResult;
    pub fn state(&self) -> &CpuState;
    pub fn capture(&self) -> CpuSnapshot;
    pub fn restore(&mut self, snapshot: &CpuSnapshot)
        -> Result<(), CpuError>;
}
\`\`\`

Критические требования:

1. \`clock\` продвигает ровно один CPU cycle.
2. На каждом цикле выполняется соответствующий bus access, если он предусмотрен микропоследовательностью. Dummy read/write — наблюдаемые операции.
3. \`step_instruction\` (если предоставлен) реализуется поверх повторных вызовов \`clock\`, а не отдельным атомарным интерпретатором.
4. Декодер total: все 256 значений имеют descriptor.
5. Таблица opcode отделена от исполнения; disassembler и анализатор используют ту же таблицу.
6. Состояние включает незавершённую микропоследовательность и все latch/temporary address, необходимые для продолжения.
7. Ricoh 2A03 не выполняет десятичную коррекцию ADC/SBC; D-флаг сохраняется как бит регистра.
8. B не является постоянным физическим флагом: образ стека для PHP/BRK отличается от IRQ/NMI.
9. IRQ — level-sensitive, NMI — edge-latched; polling и vector selection происходят в соответствующих циклах.
10. Неофициальные opcode не маскируются под NOP без явного descriptor и семантики.

Подробные последовательности циклов и группы undocumented opcodes задаются документом [CPU_MICROCODE.md](CPU_MICROCODE.md).

### 4.4. Mapper — \`nrs-mappers\`

\`\`\`rust
pub trait Mapper {
    fn cpu_read(&mut self, address: u16) -> MapperRead;
    fn cpu_write(&mut self, address: u16, value: u8);
    fn ppu_read(&mut self, address: u16) -> MapperRead;
    fn ppu_write(&mut self, address: u16, value: u8);
    fn mirroring(&self) -> Mirroring;
    fn clock_cpu(&mut self);
    fn irq_pending(&self) -> bool;
}
\`\`\`

Контракт mapper:

- mapper отвечает за банковое отображение и специфичные board side effects;
- адреса и тип доступа не должны теряться;
- CHR RAM и CHR ROM различаются по возможности записи;
- mirroring может быть фиксированным или динамическим;
- IRQ mapper передаётся в общий interrupt path;
- mapper snapshot включает все bank registers, IRQ counters/latches и прочее состояние платы.

Первый обязательный mapper — NROM. UxROM/CNROM добавляются после стабилизации mapper API. Неизвестный mapper не подменяется NROM.

### 4.5. Memory buses — \`nrs-memory\`

\`\`\`rust
pub struct CpuBusState {
    pub open_bus: u8,
    pub ram: Box<[u8; 0x800]>,
}

pub struct PpuBusState {
    pub open_bus: u8,
    pub nametable_ram: Box<[u8]>,
    pub palette_ram: [u8; 32],
}

pub struct MemorySystem {
    // CPU/PPU bus devices and cartridge mapping
}

impl CpuBus for MemorySystem {
    fn read(&mut self, address: u16, kind: CpuAccessKind) -> u8;
    fn write(&mut self, address: u16, value: u8, kind: CpuAccessKind);
}
\`\`\`

CPU address map:

| Диапазон | Назначение | Примечание |
|---|---|---|
| \`$0000-$1FFF\` | 2 KiB internal RAM | Зеркала каждые \`$0800\` |
| \`$2000-$3FFF\` | PPU registers | Зеркала каждые 8 байт; побочные эффекты регистров сохраняются |
| \`$4000-$4017\` | APU и контроллеры | Регистры APU, DMA, status, input ports |
| \`$4018-$401F\` | Test/disabled expansion area | Явно определённое поведение open bus/неподдерживаемого диапазона |
| \`$4020-$FFFF\` | Cartridge/expansion | Маршрутизация через mapper и cartridge board |

PPU address map:

| Диапазон | Назначение |
|---|---|
| \`$0000-$1FFF\` | CHR через mapper |
| \`$2000-$2FFF\` | Nametable RAM с mirroring |
| \`$3000-$3EFF\` | Зеркало nametable области |
| \`$3F00-$3FFF\` | Palette RAM и её зеркала |

Нельзя реализовывать memory map простым массивом 64 KiB: чтения PPU/APU-регистров, open bus, mapper и DMA имеют побочные эффекты и временные свойства.

### 4.6. PPU — \`nrs-ppu\`

\`\`\`rust
pub struct PpuPosition {
    pub scanline: u16,
    pub dot: u16,
    pub frame: u64,
}

pub trait PpuBus {
    fn read(&mut self, address: u16, kind: PpuAccessKind) -> u8;
    fn write(&mut self, address: u16, value: u8, kind: PpuAccessKind);
}

impl Ppu {
    pub fn clock<B: PpuBus>(&mut self, bus: &mut B) -> PpuEvents;
    pub fn cpu_read_register(&mut self, register: PpuRegister) -> u8;
    pub fn cpu_write_register(&mut self, register: PpuRegister, value: u8);
}
\`\`\`

Минимальная модель первого этапа:

- PPU registers и latch поведения;
- scroll/address write toggle, buffered reads, status side effects;
- OAM access и OAM DMA endpoint;
- nametable/palette/CHR bus;
- scanline/dot/frame progression по выбранному региону;
- vblank set/clear и NMI output timing;
- базовый framebuffer 256×240, достаточный для диагностического вывода и первичной проверки;
- snapshot включает pipeline/latches, а не только видимые регистры.

Риск: PPU rendering timing имеет множество граничных случаев. Простая «отрисовка кадра после CPU-инструкций» не является допустимой архитектурой. Рендеринг должен быть привязан к PPU dots и реальным PPU bus fetches. Точность каждого obscure rendering bug не является критерием первого релиза, но известные тестируемые базовые сценарии должны проходить.

### 4.7. APU — \`nrs-apu\`

\`\`\`rust
pub struct ApuEvents {
    pub irq: bool,
    pub dmc_dma_request: Option<DmcDmaRequest>,
    pub sample: Option<i16>,
}

impl Apu {
    pub fn clock_cpu_cycle(&mut self) -> ApuEvents;
    pub fn read_status(&mut self) -> u8;
    pub fn write_register(&mut self, address: u16, value: u8);
}
\`\`\`

Минимальная граница первого этапа:

- регистры pulse, triangle, noise и DMC;
- frame sequencer и frame IRQ;
- продвижение таймеров каналов на CPU clock;
- DMC fetch request/IRQ и контракт DMA arbitration;
- детерминированный integer mixer/sample accumulator;
- аудиосэмплы выдаются через очередь/поток, не через аудиоустройство изнутри APU.

Высокоточная аналоговая фильтрация и калибровка под конкретные ревизии не входят в первый критерий. Но регистровые side effects, frame IRQ и DMC stalls нельзя имитировать только генерацией произвольного звука.

### 4.8. Emulator orchestration — \`nrs-emulator\`

\`\`\`rust
pub struct EmulatorConfig {
    pub region: Region,
    pub trace: TraceConfig,
}

pub struct Emulator {
    // CPU, PPU, APU, memory, mapper, scheduler, input state
}

impl Emulator {
    pub fn load(image: RomImage, config: EmulatorConfig)
        -> Result<Self, EmulatorError>;

    pub fn clock_cpu_cycle(&mut self) -> Result<FrameEvents, EmulatorError>;
    pub fn run_until_frame(&mut self) -> Result<FrameOutput, EmulatorError>;
    pub fn set_input(&mut self, event: InputEvent);
    pub fn capture_snapshot(&self) -> EmulatorSnapshot;
    pub fn restore_snapshot(&mut self, snapshot: &EmulatorSnapshot)
        -> Result<(), EmulatorError>;
}
\`\`\`

Только \`nrs-emulator\` определяет порядок продвижения устройств. CPU не вызывает PPU напрямую, APU не меняет PC, PPU не управляет scheduler.

---

## 5. CPU/PPU/APU scheduler и DMA

### 5.1. Clock domains

- CPU выполняет один CPU cycle за вызов CPU clock.
- PPU продвигается в PPU dots согласно региональному рациональному отношению.
- APU получает один аппаратный CPU clock на соответствующем CPU cycle (с учётом чётности и особенностей каналов).
- Mapper может получать CPU/PPU clock hooks и наблюдать соответствующие bus accesses.

Начальные отношения CPU:PPU, используемые scheduler, задаются как рациональные коэффициенты в текущем core contract: NTSC 3:1 PPU dots на CPU cycle; PAL 16:5; Dendy 3:1. Эти коэффициенты не заменяют тестирование фаз, scanline length, odd-frame skip и различий ревизий.

### 5.2. Порядок событий одного CPU cycle

Порядок должен быть зафиксирован кодом и тестами. Предлагаемый контракт:

1. обработать запланированную CPU bus operation или stall/arbitration;
2. выполнить требуемые PPU dots согласно фазе divider, с документированным относительным порядком bus events;
3. clock APU;
4. обновить mapper IRQ/NMI inputs и линии устройств;
5. завершить CPU cycle и опубликовать trace/events.

Это предварительный порядок реализации. До прохождения timing ROM он не считается аппаратно доказанным. Если тесты выявят необходимость иного подциклового порядка, контракт меняется централизованно в scheduler, а не локальными задержками в устройствах.

### 5.3. DMA arbitration

DMA не должна реализовываться как блокирующая копия массива поверх CPU:

- OAM DMA инициируется записью в \`$4014\`;
- CPU останавливается на аппаратно определённое количество циклов (обычно 513/514 в зависимости от alignment);
- во время DMA продолжаются PPU/APU clock и необходимые bus accesses;
- DMC DMA может отбирать bus cycles и взаимодействовать с OAM DMA;
- состояние арбитра и оставшиеся stall cycles входят в snapshot.

Для первого прохода допускается реализовать OAM DMA раньше DMC DMA, но до объявления эмулятора пригодным для широкого запуска DMC stalls должны быть включены и проверены тестами.

---

## 6. Модель состояния и snapshots

### 6.1. Полнота snapshot

Snapshot — не просто копия регистров CPU. Для детерминированного продолжения он должен включать:

| Компонент | Что сохраняется |
|---|---|
| CPU | Регистры, PC, P, opcode, все latch, micro-step, interrupt polling, NMI/IRQ state, halt/reset state, cycle counter |
| CPU bus | RAM, open bus, pending side effects, bus-visible latch |
| PPU | Регистры, VRAM/OAM/palette, scanline/dot/frame, scroll/address latches, read buffer, rendering pipeline, NMI state |
| APU | Регистры, channel timers/counters, frame sequencer, length/envelope/sweep, DMC state, IRQ |
| Mapper | Bank registers, mirroring mode, IRQ counter/latch, CHR RAM и прочее изменяемое состояние |
| Scheduler | Регион, divider phase, master tick, CPU parity, текущая фаза события |
| DMA | OAM/DMC transfer phase, address/data latch, pending requests, remaining cycles |
| Input | Состояние кнопок, serial shift registers, индекс применённых событий |
| Runtime | Frame counter, deterministic audio/sample accumulator, trace sequence cursor при необходимости |

Неизменяемые PRG/CHR ROM bytes могут быть ссылкой на ROM asset по SHA-256, но snapshot должен проверять совпадение хеша и версии mapper/runtime.

### 6.2. Snapshot API и версии

\`\`\`rust
pub struct SnapshotHeader {
    pub schema_version: u32,
    pub emulator_build: String,
    pub rom_sha256: [u8; 32],
    pub mapper_id: u16,
    pub region: Region,
}

pub struct EmulatorSnapshot {
    pub header: SnapshotHeader,
    pub cpu: CpuSnapshot,
    pub ppu: PpuSnapshot,
    pub apu: ApuSnapshot,
    pub memory: MemorySnapshot,
    pub mapper: MapperSnapshot,
    pub scheduler: SchedulerSnapshot,
    pub input: InputSnapshot,
    pub dma: DmaSnapshot,
}
\`\`\`

Требования:

- snapshot несовместимой версии отклоняется явной ошибкой;
- восстановление либо атомарно успешно, либо не меняет активное состояние;
- snapshot можно снять между любыми двумя CPU cycles, включая середину opcode и DMA;
- после restore последующая trace sequence должна совпадать с uninterrupted run;
- миграция старых snapshot — отдельная версия задачи, не неявная десериализация.

В первом этапе обязателен in-memory snapshot и тест эквивалентности. Публичный долговременный бинарный формат snapshot можно стабилизировать после изменения состава pipeline state.

### 6.3. Timeline contract

Для будущего Timeline фиксируются:

- периодические snapshots;
- упорядоченный журнал входных событий с абсолютным tick/frame index;
- deterministic replay от ближайшего snapshot;
- отдельные ветви при изменении входа из прошлого;
- запрет менять уже сохранённую историю без создания новой ветви.

UI, визуальная шкала, быстрый scrub и хранение большого числа snapshot — не часть первого этапа.

---

## 7. Trace model и связь эмулятора с анализатором

### 7.1. Событие трассы

\`\`\`rust
pub struct BusTraceEvent {
    pub sequence: u64,
    pub cpu_cycle: CpuCycle,
    pub ppu_position: PpuPosition,
    pub master_tick: MasterTick,
    pub address: u16,
    pub value: u8,
    pub direction: BusDirection,
    pub access: CpuAccessKind,
    pub pc: u16,
    pub opcode: Option<u8>,
    pub mapper_context: MapperContext,
}
\`\`\`

Трасса должна различать opcode fetch, operand read, data read/write, dummy read/write, stack access, vector fetch, DMA access и другие значимые типы. Нельзя сводить все чтения к одному событию Read.

Дополнительно instruction event может агрегировать:

- стартовый PC;
- opcode и bytes;
- cycle range;
- descriptor ID;
- конечный PC;
- регистровые изменения;
- связь с bus event sequence.

Агрегат не заменяет сырую bus trace.

### 7.2. Поток данных

\`\`\`text
ROM bytes
   |
nrs-rom ----> Cartridge metadata / bank map
   |                         |
   +----> nrs-analysis <------+
                ^
                | opcode metadata + static seeds
                |
nrs-emulator --> TraceSink --> TraceEvent stream
                |
                +--> snapshots / frame hashes
                |
                v
             nrs-ir
                |
         CFG + observed edges
                |
       future Blueprint editor
\`\`\`

Trace sink должен быть сменным:

- \`NullTraceSink\` для производительности;
- \`RingBufferTraceSink\` для интерактивной диагностики;
- \`FileTraceSink\` для воспроизводимых тестов;
- \`AnalysisTraceSink\` для передачи событий анализатору.

Полный trace может быть очень большим. Runtime не должен бесконтрольно хранить его весь в RAM. Нужны фильтры, лимиты, потоковая запись и явное сообщение о потерянных событиях.

### 7.3. Анализ наблюдений

Анализатор различает:

- **факт:** CPU действительно прочитал адрес X со значением Y на цикле N;
- **декодирование:** байты по адресу X соответствуют opcode Z в конкретном mapping context;
- **наблюдаемая связь:** переход на адрес Y произошёл во время конкретного запуска;
- **гипотеза:** адрес X вероятно является началом функции или таблицы;
- **пользовательская метка:** имя/комментарий, введённые человеком.

Наблюдаемая трасса подтверждает конкретный запуск и состояние банков, но не доказывает, что все возможные пути игры исследованы.

---

## 8. Промежуточное представление IR

### 8.1. Требования

IR должен быть:

- lossless относительно исходных байтов каждого декодированного фрагмента;
- bank-aware: логический CPU address недостаточен без контекста mapper;
- пригодным для CFG, data-flow и динамических наблюдений;
- независимым от UI;
- версионируемым и сериализуемым;
- расширяемым пользовательскими annotations без изменения машинного слоя.

### 8.2. Основные сущности

\`\`\`rust
pub struct ProgramIr {
    pub schema_version: u32,
    pub source: RomSourceRef,
    pub address_spaces: Vec<AddressSpace>,
    pub regions: Vec<CodeRegion>,
    pub instructions: Vec<InstructionIr>,
    pub blocks: Vec<BasicBlockIr>,
    pub edges: Vec<ControlFlowEdge>,
    pub symbols: Vec<SymbolAnnotation>,
    pub observations: Vec<ExecutionObservation>,
}

pub struct InstructionIr {
    pub id: InstructionId,
    pub logical_address: u16,
    pub physical_offset: Option<u32>,
    pub bank_context: BankContext,
    pub raw_bytes: Vec<u8>,
    pub opcode: u8,
    pub descriptor_id: OpcodeDescriptorId,
    pub operands: DecodedOperands,
    pub provenance: Provenance,
    pub confidence: Confidence,
}
\`\`\`

Рекомендуемые типы узлов:

| Узел | Назначение |
|---|---|
| \`Instruction\` | Декодированная машинная инструкция |
| \`OpaqueBytes\` | Байты неизвестного/неподдержанного участка |
| \`BasicBlock\` | Последовательность инструкций с одним входом |
| \`CallSite\` | Вызов подпрограммы |
| \`Branch\` | Условный/безусловный переход |
| \`InterruptVector\` | Вход через NMI/RESET/IRQ vector |
| \`MemoryAccess\` | Наблюдаемые чтения/записи |
| \`MapperTransition\` | Изменение bank mapping |
| \`Annotation\` | Имя, комментарий, гипотеза, цвет/группа для UI |
| \`UnknownSemantic\` | Декодируемый opcode или участок с неподдержанной/неустановленной семантикой |

### 8.3. Адрес и идентичность

Один и тот же логический адрес может отображать разные PRG banks. Поэтому ключ инструкции не может быть только \`u16\`.

Идентичность должна учитывать как минимум:

- hash ROM;
- logical address;
- физический PRG offset либо bank mapping context;
- исходные bytes/opcode;
- версию descriptor table.

Если bank mapping неизвестен или динамически меняется, IR сохраняет несколько вариантов контекста и не склеивает их автоматически.

### 8.4. Control-flow edges

Типы связей:

- \`Fallthrough\`;
- \`ConditionalTaken\`;
- \`ConditionalNotTaken\`;
- \`Jump\`;
- \`Call\`;
- \`Return\`;
- \`InterruptEntry\`;
- \`ObservedTransition\`;
- \`InferredTransition\`;
- \`BankSwitch\`;
- \`UnknownTarget\`.

У каждой связи есть provenance и confidence. Статически вычисленная цель branch не должна выглядеть как фактически пройденная трассой.

### 8.5. Blueprint-first invariant

Для каждого участка, который анализатор пометил как исполняемый, должен существовать видимый IR node или явно обозначенный opaque node. Не допускается ситуация, когда часть исполняемого кода молча пропала из графа.

При этом **не каждый байт ROM обязан быть нодой исполняемого графа**: графика, аудиоданные, таблицы и padding должны быть представлены как data/unknown regions, а не как инструкции.

### 8.6. Двусторонний pipeline

\`\`\`text
ROM bytes
   |
decode + map + trace
   v
Lossless machine IR
   |
annotations / grouping / high-level views
   v
Blueprint model
   |
lowering / validation / layout
   v
Machine IR + relocation plan
   |
assembler + linker (future)
   v
ROM image
\`\`\`

На первом этапе реализуется ROM → metadata → static/dynamic IR. Обратный путь ограничен сохранением неизменённых байтов и проектированием интерфейса lowering. Полноценная компиляция изменённых графов в ROM не заявляется как готовая функция.

---

## 9. Поток импорта ROM и анализа

### 9.1. Импорт

1. Прочитать исходный файл без изменения.
2. Рассчитать SHA-256 и сохранить размер/имя источника.
3. Разобрать iNES/NES 2.0 header.
4. Проверить длину, trainer, PRG/CHR regions и арифметику смещений.
5. Построить \`RomImage\` и cartridge metadata.
6. Определить mapper/submapper и наличие реализации.
7. Создать проектную запись ROM asset.
8. Построить первичную карту PRG/CHR banks и векторов.
9. Сохранить исходные байты или ссылку на неизменяемый asset с проверкой hash.

Импорт не должен запускать ROM автоматически. Анализ и исполнение — отдельные пользовательские действия.

### 9.2. Статический анализ

Начальные seeds:

- reset vector;
- NMI vector;
- IRQ/BRK vector;
- дополнительные адреса, явно заданные пользователем;
- адреса, подтверждённые trace.

Алгоритм:

1. получить mapping context для стартового адреса;
2. декодировать opcode общей таблицей CPU;
3. вычислить длину и статически известные цели;
4. построить fallthrough/branch/call edges;
5. продолжать worklist до уже посещённого состояния, границы области или неоднозначности;
6. учитывать bank-switch writes и альтернативные mapping contexts;
7. сохранять конфликты code/data и неоднозначности, не скрывая их.

Линейный sweep всего PRG как кода запрещён в качестве единственного алгоритма: он неизбежно ошибочно декодирует данные.

### 9.3. Динамический анализ

1. Создать runtime из ROM и региона.
2. Сбросить CPU через reset sequence.
3. Применить input events с точными frame/cycle positions.
4. Собирать instruction и bus trace.
5. Сохранять контрольные snapshots/frame hashes.
6. Передать trace анализатору.
7. Сопоставить observed PCs и transitions со статическим IR.
8. Пометить недостижимые/неоднозначные участки как «не наблюдались», а не как «неисполняемые».

### 9.4. Повторяемость запуска

Для каждого динамического анализа сохранять:

- SHA-256 ROM;
- runtime build/version;
- region;
- начальный snapshot или правило reset;
- точную последовательность input events;
- frame/cycle limit;
- конфигурацию trace;
- итоговый state/frame hashes.

Без этих параметров трасса не является полностью воспроизводимым артефактом.

---

## 10. Проектный формат \`.nrsproj\`

В первом этапе фиксируется концептуальная структура, но формат должен оставаться версионируемым:

\`\`\`text
project.nrsproj/
├── project.json
├── assets/
│   └── rom/
│       └── <sha256>.nes
├── analysis/
│   ├── program-ir.json
│   └── reports/
├── traces/
│   └── <run-id>.trace
├── snapshots/
│   └── <snapshot-id>.nrsnap
└── user/
    └── annotations.json
\`\`\`

Допускается и единый файл-контейнер в будущем. Важно разделять:

- immutable source asset;
- regenerable analysis cache;
- user annotations;
- execution traces;
- snapshots;
- project settings.

Потеря cache не должна приводить к потере пользовательских меток. Пересчёт анализа не должен менять исходный ROM. Любая запись сопровождается версией схемы.

---

## 11. Тестовая стратегия

Тесты строятся слоями. Прохождение unit tests не является доказательством полной NES compatibility.

### 11.1. Unit tests по crates

| Crate | Обязательные тесты |
|---|---|
| \`nrs-core\` | Rational divider, phase restore, региональные коэффициенты, overflow boundaries |
| \`nrs-rom\` | iNES/NES2 headers, trainer offsets, CHR RAM, truncated/oversized input, invalid mapper metadata |
| \`nrs-cpu\` | Все addressing modes, flags, stack, wraparound, page crossing, unofficial opcodes, IRQ/NMI/RESET |
| \`nrs-mappers\` | PRG/CHR mapping, bank switching, mirroring, mapper snapshots |
| \`nrs-memory\` | RAM/register mirrors, open bus, side effects, PPU bus mapping |
| \`nrs-ppu\` | Register latches, buffered reads, vblank/NMI timing, address mirrors, OAM |
| \`nrs-apu\` | Register semantics, frame sequencer, IRQ, timer progression, DMC request |
| \`nrs-emulator\` | Scheduler order, DMA stalls, input determinism, snapshot restore |
| \`nrs-analysis\` | CFG edges, bank context, ambiguous decode, trace correlation |
| \`nrs-ir\` | Stable IDs, serialization round-trip, provenance preservation |

### 11.2. CPU conformance

Обязательные источники тестов:

- SingleStepTests 6502 JSON vectors;
- nestest register/cycle log;
- Klaus Dormann functional tests;
- blargg NES CPU test ROMs;
- AccuracyCoin CPU tests;
- отдельные bus-trace fixtures NRS для dummy cycles, page crossing, stack и interrupt polling.

Для тестовых ROM необходимо фиксировать происхождение, лицензию/условия распространения и ожидаемый результат. В репозитории не следует без проверки добавлять ROM-файлы с неясными правами; допустимы инструкции по получению и локальные fixture, созданные проектом.

### 11.3. PPU/APU и системные тесты

- PPU register/status tests и vblank/NMI timing ROMs;
- sprite/background базовые тесты и framebuffer golden hashes;
- APU register/frame IRQ tests;
- DMC DMA interaction tests;
- OAM DMA alignment/stall tests;
- mapper-specific tests для каждого реализованного mapper;
- региональные timing tests для NTSC/PAL там, где fixture доступна.

### 11.4. Snapshot/replay equivalence

Ключевой тест:

1. выполнить ROM до выбранного CPU cycle;
2. сохранить snapshot;
3. продолжить M циклов и записать state + bus trace + frame hash;
4. восстановить snapshot в новом runtime;
5. повторить те же M циклов;
6. сравнить все события и контрольные значения побайтно.

Повторять тест для точек посреди opcode, branch, interrupt entry, OAM DMA и DMC DMA. Snapshot, который восстанавливает только состояние между кадрами, недостаточен для Timeline.

### 11.5. Property/fuzz tests

- fuzz ROM header parser и расчёта размеров;
- случайные последовательности bus read/write в тестовой memory map;
- случайные snapshots/restore с проверкой инвариантов;
- случайные opcode streams в изолированной RAM с лимитом циклов и проверкой отсутствия panic/undefined state;
- IR serialization round-trip и проверка уникальности идентификаторов.

---

## 12. Критерии приёмки первой рабочей версии

Первый этап принимается только при выполнении всех обязательных критериев ниже.

### Gate A — workspace и импорт

- [ ] \`cargo fmt --all -- --check\` проходит.
- [ ] \`cargo check --workspace\` проходит без ошибок.
- [ ] \`cargo test --workspace\` проходит.
- [ ] Валидные iNES ROM разбираются с корректными offsets, trainer и PRG/CHR lengths.
- [ ] Поддерживаемый поднабор NES 2.0 разбирается корректно.
- [ ] Повреждённые и усечённые ROM дают понятную ошибку, не panic.
- [ ] SHA-256 исходного ROM сохраняется в asset metadata.
- [ ] Неподдерживаемый mapper явно отклоняется при запуске.

### Gate B — CPU и bus

- [ ] Таблица декодирует все 256 opcode bytes без пустых entries.
- [ ] Официальные инструкции проходят выбранные функциональные тесты.
- [ ] Стабильные undocumented opcode имеют отдельную семантику и тесты.
- [ ] Нестабильные opcode имеют явную descriptor/policy и не маскируются под NOP.
- [ ] Один вызов \`clock\` продвигает ровно один CPU cycle.
- [ ] Bus trace подтверждает адрес, направление, значение и цикл для тестов addressing modes.
- [ ] Dummy reads/writes, page crossing и zero-page wrap проверяются.
- [ ] Reset, BRK, IRQ, NMI и stack sequences проверяются.
- [ ] nestest и согласованный набор CPU test ROMs проходят с документированными исключениями.

### Gate C — PPU/APU/memory/scheduler

- [ ] CPU/PPU memory maps и зеркала покрыты тестами.
- [ ] NROM ROM может быть запущен с reset vector.
- [ ] PPU timing продвигается независимо от границ CPU-инструкций.
- [ ] Vblank/NMI и PPU register side effects проходят тесты.
- [ ] Базовый framebuffer выдаётся runtime.
- [ ] APU frame sequencer, status/IRQ и базовые таймеры работают детерминированно.
- [ ] OAM DMA имеет проверенные stall/alignment semantics.
- [ ] DMC DMA либо реализован и протестирован, либо первый релиз явно помечен как ограниченный и не заявляется как совместимый с ROM, требующими DMC.
- [ ] Один и тот же input sequence при одинаковой конфигурации даёт одинаковые frame/state hashes.

### Gate D — анализ и IR

- [ ] Статический анализ начинает обход с reset/NMI/IRQ vectors.
- [ ] CFG учитывает conditional branch, jump, call/return и неизвестные цели.
- [ ] Адреса PRG bank не сливаются только по 16-bit logical address.
- [ ] Каждый декодированный фрагмент сохраняет исходные bytes.
- [ ] Неизвестные/неоднозначные участки сохраняются как opaque/unknown.
- [ ] Trace events связываются с IR nodes без потери provenance.
- [ ] IR сериализуется и загружается с проверкой schema version.
- [ ] Повторный анализ не уничтожает пользовательские annotations.

### Gate E — snapshots и воспроизводимость

- [ ] Snapshot содержит CPU microstate, PPU/APU, memory, mapper, scheduler, DMA и input state.
- [ ] Restore возможен посреди инструкции и DMA.
- [ ] Snapshot/replay equivalence проходит на контрольных сценариях.
- [ ] Несовместимый ROM hash/runtime/schema вызывает явную ошибку.
- [ ] Трасса содержит достаточно данных для воспроизводимого анализа.

### Ограничение приёмки

Прохождение Gate A–E означает готовность **первого headless analysis runtime**, а не полную совместимость со всей библиотекой коммерческих NES игр. Поддержка каждого mapper, региональная точность, редкие PPU/APU quirks и все нестандартные платы должны иметь отдельные compatibility reports.

---

## 13. План реализации и зависимости работ

| Подэтап | Результат | Зависимости |
|---|---|---|
| 1.1 Workspace contracts | Уточнены общие типы, ошибки, clock и bus contracts | Уже существующий \`nrs-core\` |
| 1.2 ROM parser | iNES/NES2 parsing, ROM metadata, hash | 1.1 |
| 1.3 CPU opcode metadata | Полная таблица 256 opcode и decoder tests | 1.1 |
| 1.4 CPU microsequencer | Cycle-by-cycle official/unofficial execution, interrupts | 1.3 |
| 1.5 Memory + NROM | CPU/PPU buses, RAM, cartridge routing | 1.2, 1.4 |
| 1.6 PPU baseline | Registers, timing, vblank/NMI, framebuffer | 1.5 |
| 1.7 APU baseline + DMA | Registers, timers, frame sequencer, DMA integration | 1.5 |
| 1.8 Emulator scheduler | Deterministic machine clock, input, snapshots | 1.4–1.7 |
| 1.9 Trace pipeline | Bus/instruction event stream, sinks | 1.4–1.8 |
| 1.10 Static analyzer + IR | Vector seeds, bank-aware CFG, provenance | 1.2, 1.3, 1.9 |
| 1.11 CLI integration | Inspect/run/trace/analyze commands | 1.2–1.10 |
| 1.12 Conformance gate | Test ROM report, reproducibility, acceptance checklist | Все предыдущие |

Рекомендуется интегрировать каждый подэтап отдельными небольшими коммитами. Не следует одновременно писать весь PPU, APU, CPU и analyzer в одном большом изменении: это усложнит локализацию timing regressions.

---

## 14. Будущие интерфейсы: закладываем сейчас, не реализуем сейчас

### 14.1. Blueprint editor

IR должен иметь стабильные node IDs, graph edges, annotations и provenance. UI сможет отображать машинные узлы, группировать basic blocks и показывать более высокоуровневые представления. Первый этап не реализует drag-and-drop graph editor, визуальный debugger и библиотеку игровых нод.

### 14.2. Split Screen

Архитектура должна позволять создать два независимых \`Emulator\` экземпляра:

\`\`\`rust
pub struct ComparisonSession {
    pub original: Emulator,
    pub reconstructed: Emulator,
    pub shared_input: InputTimeline,
}
\`\`\`

Оба runtime получают идентичные input events и имеют сопоставимые frame/cycle counters. Визуальное сравнение, синхронная пауза, diff framebuffer и divergence viewer отложены.

### 14.3. Timeline

Snapshot + deterministic input log + trace cursor образуют основу Timeline. Для быстрого перемещения можно будет использовать периодические checkpoints и replay от ближайшей точки. UI timeline, ветвление, scrubbing и управление памятью snapshots отложены.

### 14.4. ROM compiler

Обратный pipeline потребует:

- машинно-точного IR;
- assembler для 6502/undocumented opcodes;
- bank-aware linker;
- relocation records;
- сохранения mapper constraints и vectors;
- контроля размера сегментов;
- проверки round-trip.

Первый этап не обещает компиляцию произвольного Blueprint в ROM. Обязательная будущая гарантия — untouched regions должны сохранять исходные байты; изменённые regions должны проходить явную валидацию и иметь отчёт о relocation/переполнении.

### 14.5. Standalone export

Standalone продукт должен включать ROM/game package и совместимый NRS runtime для целевой платформы. Это не означает автоматическую нативную компиляцию машинного кода 6502 в x86/ARM. Формат package, платформенные backend и UI запуска проектируются позднее.

---

## 15. Основные технические риски

| Риск | Последствие | Снижение риска |
|---|---|---|
| Неверные CPU dummy cycles/interrupt polling | Неправильные трассы и несовместимость игр | Cycle-level tests, nestest, AccuracyCoin, bus trace |
| Неполный snapshot | Timeline/replay расходятся после restore | Snapshot equivalence на каждом типе microstate |
| Неправильный scheduler ordering | Гонки CPU/PPU/APU и региональные ошибки | Один scheduler owner, timing fixtures, фиксированный event order |
| Ошибки mapper/bank context | CFG объединяет разные участки кода | Physical offsets + mapping context в IR |
| Code/data ambiguity | Ложные функции и графы | Confidence/provenance, opaque regions, ручная коррекция |
| Неполная PPU timing model | ROM зависает/не отображает ожидаемое | Этапные PPU test ROMs, явный compatibility report |
| APU/DMC stalls | Timing-sensitive игры расходятся | DMC DMA arbitration и тесты совместного DMA |
| Чрезмерная трассировка | Память и производительность | Streaming sinks, ring buffer, filters, trace limits |
| Нестабильный IR schema | Невозможно открыть старые проекты | schema versioning и migration tests |
| Слишком ранний UI | UI закрепляет неверные контракты | Сначала headless runtime и IR invariants |

Особо важно: «все opcode декодируются» не равнозначно «CPU совместим». Точно так же успешный запуск одной игры не доказывает корректность PPU/APU или mapper support.

---

## 16. Definition of Done

Этап 1 завершён, когда:

1. все обязательные crates имеют определённые публичные контракты и тесты;
2. ROM import не теряет исходные данные и сообщает о неподдерживаемых форматах/mapper;
3. CPU работает по циклам и проходит согласованный набор функциональных и bus-trace тестов;
4. memory, NROM, базовый PPU/APU и scheduler образуют воспроизводимый headless runtime;
5. snapshots восстанавливают исполнение без расхождения;
6. trace pipeline сохраняет машинные факты и связывает их с IR;
7. статический анализ строит bank-aware граф с явной неопределённостью;
8. IR сохраняет bytes, mapping context, provenance и user annotations;
9. CLI позволяет импортировать ROM, вывести metadata, запустить ограниченное число циклов/кадров, записать trace и выполнить первичный анализ;
10. все Gate A–E имеют отчёт о прохождении и список известных исключений.

**Итоговая формулировка:** первый этап NRS — это не готовый визуальный редактор и не универсальный NES emulator. Это детерминированное, тестируемое и трассируемое ядро reverse engineering, которое сохраняет точную машинную информацию и создаёт основу для Blueprint-first реконструкции, Timeline, Split Screen и двусторонней сборки ROM.
