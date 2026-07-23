# dkms-bindings / bindings/wasm — analiza pod przygotowanie do rozmów technicznych

Analiza wykonana 2026-07-23 na forku `olichwiruk/dkms-bindings`, branch
`claude/rust-wasm-bindings-prep-xfhf3d` (baza: `master` = `d8827a7`).
Numery linii w cytatach „przed refaktorem" odnoszą się do stanu z `master`;
kod „po refaktorze" to commit `5f54a9a` na tym branchu.

Dokument celowo nie lukruje — sekcje „słabe punkty" są po to, żebyś na rozmowie
wyprzedził pytanie rekrutera, a nie żeby cię pogrążyć.

---

## 1. Zweryfikowane fakty (git log / git blame, nie szacunki)

| Fakt | Wartość zweryfikowana |
|---|---|
| Autorstwo `bindings/wasm` | **100%** — `git blame -w -C` na każdym pliku: wszystkie linie autorstwa `Marcin Olichwiruk <21108638+olichwiruk@users.noreply.github.com>` |
| `git blame -w -C src/lib.rs` | **601/601 linii** twojego autorstwa (stan `master`) |
| Cały `src/` + `Cargo.toml` | 3 965 linii src + 62 linie Cargo.toml — wszystkie twoje |
| Founding commit | `d1fc637` — **2025-07-24 23:32 +0200**, „feat: add wasm bindings", 6 032 insercje (w tym lib.rs 574 linie) |
| Liczba commitów dotykających `bindings/wasm` | **10** (wszystkie twoje) |
| Follow-upy po founding commicie | **9**: 1× 2025-08-27 (`dd8349e`, profile.release), 5× 2025-09-04 (m.in. `f6ccbf4` range-processing w `process_kel`, `e6ff5fa` query KEL od ostatniego sn, `83fa84a` refaktor `From<VcState>`), 3× 2025-09-05 (`eb8fc78` flagi wasm-opt, `d27fded` release 0.1.1, `e923c2c`) |
| `build.rs` | **nie istnieje** (i nie jest potrzebny — brak codegen/link-time zależności) |
| Testy | **zero** — brak `#[test]`, `#[cfg(test)]`, `wasm_bindgen_test` w całym module. To pierwsza rzecz, którą wytknie każdy senior. Nie ukrywaj tego; miej gotową odpowiedź (patrz pytanie 19 w banku pytań) |
| Dokumentacja | **zero rustdoc** na publicznym API (`///` nie występuje w lib.rs); jedyny doc-comment w `in_memory/mod.rs:68` |
| CI (`ci-wasm.yml`) | job `check` robi `cargo check --all-features` **bez** `--target wasm32-unknown-unknown` (kompiluje pod host x86_64 — patrz sekcja 4 dlaczego to „przechodzi, ale nie sprawdza tego, co trzeba"); publikacja: `wasm-pack build --target web --release` + `npm publish` na tag `wasm*` |
| Dystrybucja | npm, pakiet `dkms-wasm` 0.1.1, `release.toml` z `publish = false` (cargo-release tylko taguje `wasm-{{version}}`) |

Weryfikacja twierdzeń z briefingu:

- **„100% mojego autorstwa"** — potwierdzone, bez gwiazdek.
- **„~1 unwrap / 20 linii"** — potwierdzone **dla `lib.rs`**: 31 wystąpień / 601
  linii = **1/19,4**. Dla całego modułu jest łagodniej: 112 wystąpień / 3 965
  linii raw = **1/35,4** (licząc tylko niepuste linie bez komentarzy: 112/3 381 =
  1/30,2). Na rozmowie podawaj precyzyjnie: „na granicy FFI miałem ~1/20,
  w całym module ~1/30–1/35" — pełna inwentaryzacja w sekcji 3.

---

## 2. Analiza koncept-po-koncepcie, plik po pliku

### 2.1 `Cargo.toml` — profil, ABI i dystrybucja

```toml
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["--enable-bulk-memory", "--enable-nontrapping-float-to-int"]

[profile.release]
strip = true
opt-level = "z"
lto = true

[lib]
crate-type = ["cdylib"]
```

**a) Co i dlaczego.** `cdylib` = artefakt bez rustowego ABI, tylko eksporty C-like,
z których wasm-bindgen generuje glue JS. `opt-level = "z"` + `lto` + `strip`
to klasyczny profil „minimalny rozmiar .wasm" — w dystrybucji przez przeglądarkę
rozmiar binarki to koszt każdego page-load, więc świadomie oddajesz kilka–kilkanaście
procent szybkości za mniejszy download. Flagi `wasm-opt` włączają post-MVP
feature'y WebAssembly (bulk-memory = `memory.copy/fill`, nontrapping-float-to-int),
bo LLVM je emituje, a starszy wasm-opt bez tych flag odrzuca taki moduł.

Kluczowy niuans: **`wasm-bindgen = "=0.2.92"` jest przypięte na sztywno**, a CI
instaluje `wasm-bindgen-cli --version 0.2.92`. To nie pedanteria — glue JS
generuje CLI, a ABI między crate'em a CLI **musi być identyczne co do patcha**,
inaczej build wybucha. To jest dobra odpowiedź na pytanie „po co pinować
zależność na `=`".

Drugi niuans: sekcje

```toml
[dependencies.getrandom]
features = ["js"]
[dependencies.uuid]
features = ["v4", "js"]
```

wyglądają na nieużywane (w `src/` nie ma `use uuid`/`use getrandom`), ale to
**feature-unification shim**: zależności przechodnie (rand w `ed25519-dalek`,
uuid gdzieś w stacku KERI) potrzebują feature `js`, żeby na wasm32 brać entropię
z `crypto.getRandomValues` zamiast syscalli. Bez tego `OsRng` panikuje w runtime.
Umiej to wytłumaczyć — to częsty „gotcha" wasm.

**b) Słabe punkty (nazwane wprost).**
- `idb = "=0.6.3"` jest **martwą zależnością** — w `src/` nie ma ani jednego
  `idb::`; cały IndexedDB robiony jest ręcznie przez `web-sys`. Wygląda na
  pozostałość po podejściu „użyję wrappera", z którego się wycofałeś. Wywal albo
  migruj na niego (patrz 2.3 — migracja rozwiązałaby 80% callback-hella).
- `cesrox = "0.1.6"` w lockfile rozwiązuje się do 0.1.7 — pinowanie niespójne
  z filozofią `=0.2.92` obok.

**c) Pytanie rekrutera:** „Dlaczego `opt-level = "z"`, skoro to kod kryptograficzny —
nie boisz się o wydajność podpisów?"
**Modelowa odpowiedź:** Podpis Ed25519 to pojedyncze wywołanie na interakcję
użytkownika — mikrosekundy vs. setki ms sieci do watchera; bottleneck jest w I/O.
Za to każdy KB .wasm płacimy na każdym load. Gdyby profiling pokazał hot-path
w krypto, wybiórczo podniósłbym optymalizację (`[profile.release.package.ed25519-dalek] opt-level = 3`),
nie globalnie.

---

### 2.2 `src/lib.rs` — granica FFI

#### Koncept 1: eksport klas przez `#[wasm_bindgen]` i model własności przez ABI

```rust
#[wasm_bindgen]
pub struct JsIdentifier {
    inner: Identifier<Database>,
    db: Arc<Database>,
    alias: String,
    signer: Arc<Signer>,
    watcher_oobi: Option<LocationScheme>,
}
```

**a)** `#[wasm_bindgen]` na strukturze generuje po stronie JS klasę-uchwyt:
JS trzyma **wskaźnik do obiektu w pamięci liniowej wasm**, nie kopię danych.
Własność jest po stronie Rust; JS ma „kapability" z metodami. Metody `&self`/
`&mut self` są sprawdzane w runtime glue (wasm-bindgen wstawia dynamiczne
sprawdzenie „czy obiekt nie jest już skonsumowany/pożyczony"). Zwrócenie
`JsIdentifier` z `incept()` **przenosi własność do JS** — bez wywołania `.free()`
(lub bez `weak-refs`/FinalizationRegistry) obiekt żyje w pamięci wasm do końca
życia strony. Istotne: pola zawierają `Arc<Signer>` z **materiałem klucza
prywatnego** — patrz „słabe punkty".

Konstrukcja `impl JsIdentifier { pub fn new(...) }` w **osobnym, nieoznakowanym**
bloku impl to świadomy trik: `new` jest publiczne dla Rusta (woła je
`JsController`), ale nie jest eksportowane do JS — JS nie może sobie stworzyć
`JsIdentifier` z powietrza. Dobre, umiej to nazwać (kontrolowany konstruktor
przez fabrykę `JsController::incept/load_identifier`).

**b) Alternatywy:** (1) nie eksportować obiektów, tylko funkcje wolne +
identyfikatory-stringi (stateless API) — prostsze ABI, ale każda operacja
płaci parsowanie i lookup; (2) serializować cały stan do JS (serde-wasm-bindgen)
— zero uchwytów, ale sekrety lądują w JS heap i tracisz enkapsulację;
(3) obecny model uchwytów — najlepszy dla obiektów z sekretami i tożsamością,
kosztem ręcznego zarządzania życiem (`free()`).

**c) Pytanie:** „Co się stanie, gdy JS wywoła metodę na `JsIdentifier` po tym,
jak w innym miejscu wywołał na nim `free()`?"
**Odpowiedź:** Glue wasm-bindgen rzuci JS-owy błąd „null pointer passed to rust"
— wskaźnik jest zerowany przy free, więc nie ma use-after-free w pamięci wasm;
to runtime'owy odpowiednik borrow-checkera na granicy ABI. Dlatego API
projektuje się tak, żeby JS nie musiał ręcznie zarządzać życiem obiektów
krótkotrwałych.

#### Koncept 2: `Arc` w jednowątkowym wasm i skąd się bierze

```rust
use std::sync::Arc;
...
db: Arc<Database>,
signer: Arc<Signer>,
```

**a)** W wasm32 bez wątków `Rc` byłby „wystarczający", a `Arc` płaci za atomiki.
Ale wybór nie jest twój: `keri_sdk::Controller::new(Arc<D>, Arc<D>)` i trait
bounds SDK wymagają `Send + Sync + Arc`. To jest **host-driven constraint** —
i dokładnie z tego samego powodu w warstwie DB są `unsafe impl Send/Sync`
(patrz 2.3). Na wasm32 atomiki kompilują się do zwykłych operacji, więc koszt
realny ≈ 0.

**b) Alternatywy:** forkować SDK i parametryzować bounds (feature `single-thread`),
albo trait-alias `MaybeSend` (wzorzec z ekosystemu: `#[cfg(target_arch = "wasm32")] trait MaybeSend {}`).
Trade-off: utrzymanie forka vs. `unsafe` w bindingu.

**c) Pytanie:** „Masz `Arc<Mutex<T>>` czy `Rc<RefCell<T>>` w tym module — i dlaczego?"
**Odpowiedź:** Oba, świadomie w różnych miejscach: `Arc` tam, gdzie wymusza to
API keri-sdk (Controller/Signer), `Rc<RefCell>`/`Rc<Cell>` w warstwie IndexedDB,
gdzie stan współdzielą **closury JS-owych callbacków** — one i tak są `!Send`,
więc `Rc` jest uczciwszy, a `RefCell` daje runtime borrow-check tam, gdzie
statyczny nie sięga (callbacki o nieprzewidywalnej kolejności). Ryzyko:
`BorrowMutError` w runtime zamiast błędu kompilacji — akceptowalne, bo panika
w callbacku nie zrywa niezmienników (dane w toku są w `pending_operations`).

#### Koncept 3: async na granicy — Future ↔ Promise

```rust
pub async fn add_watcher(&mut self, url: String) -> Result<(), WasmError> { ... }
```

**a)** `async fn` w `#[wasm_bindgen]` impl-bloku staje się po stronie JS metodą
zwracającą **Promise**. Wasm nie ma własnego runtime'u — egzekutorem jest pętla
zdarzeń przeglądarki; wasm-bindgen-futures rejestruje kontynuacje jako mikrotaski.
Stąd też retry-backoff w `query_kel` nie może zrobić `thread::sleep` — używa
`gloo_timers::future::TimeoutFuture` (opakowany `setTimeout`):

```rust
gloo_timers::future::TimeoutFuture::new(delay.as_millis() as u32).await;
delay *= 2;
```

To jest ładny przykład na rozmowę: **exponential backoff (1s→2s→4s→8s→16s, 5 prób)
zaimplementowany na cudzym event loopie**.

**b) Słaby punkt, nazwany wprost:** po 5 nieudanych próbach pętla **zwraca
`Ok("")`** — pusty string wędruje do `process_kel`, a tam (patrz Koncept 5)
parser na pustym wejściu zwraca 0 zdarzeń i **cicho „sukces"**. Verify wtedy
orzeka na stanie sprzed query. Powinno być `Err(WatcherUnavailable)` po
wyczerpaniu prób. Nie poprawiłem tego w refaktorze (zmiana semantyki, nie tylko
obsługi błędów) — miej to przygotowane jako „co bym zrobił w następnym kroku".

**c) Pytanie:** „`add_watcher` bierze `&mut self` i jest async — co się stanie,
gdy JS wywoła ją dwa razy pod rząd bez await?"
**Odpowiedź:** Glue wasm-bindgen przy drugim wywołaniu rzuci błąd („recursive
use of an object..."), bo obiekt jest oznaczony jako pożyczony mutowalnie na
czas trwania Promise. To dobrze: race na `self.watcher_oobi` jest niemożliwy;
JS musi serializować wywołania. Świadomy koszt: API nie jest reentrantne.

#### Koncept 4: serializacja przez granicę — trzy różne strategie w jednym pliku

```rust
pub async fn verify(
    &self,
    identifier: &JsIdentifier,   // uchwyt (wskaźnik, zero kopiowania)
    oobi_array: JsValue,         // strukturalne JS → serde_wasm_bindgen
    message: String,             // string CESR → parsowanie cesrox
) -> Result<JsValue, WasmError>
```

**a)** W jednej sygnaturze masz trzy modele przekazywania danych:
referencję do obiektu wasm, dynamiczny `JsValue` deserializowany
`serde_wasm_bindgen::from_value` (bez rundy przez JSON string — chodzi po
strukturach JS bezpośrednio) i surowy string domenowy (CESR), bo KERI i tak
definiuje format drutowy tekstowo. Wynik wraca jako ręcznie budowany obiekt:

```rust
impl From<VerificationResult> for JsValue {
    fn from(val: VerificationResult) -> Self {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"verified".into(), &JsValue::from_bool(val.verified))
            .expect("setting verified failed");
        ...
```

**b) Alternatywy i trade-offy:** (1) JSON string w obie strony — najprostsze,
podwójne parsowanie, zero typów w TS; (2) `serde-wasm-bindgen` — bez JSON rundy,
ale typ po stronie TS to `any`; (3) **tsify** (`#[derive(Tsify)]`) — generuje
deklaracje TS dla struktur serde: dostajesz typowane API w `d.ts`. Dziś
`verify` zwraca nietypowany obiekt i przyjmuje nietypowany `oobi_array` — brak
tsify to realny brak, nazwij go sam, zanim zrobi to rekruter frontendowy.
Ręczny `Reflect::set` jest za to dobrym dowodem, że rozumiesz, co generatory
robią pod spodem.

**c) Pytanie:** „Czemu `oobi_array` to `JsValue`, a nie `Vec<JsOobi>` albo string?"
**Odpowiedź:** OOBI przychodzi z JS jako tablica obiektów; `serde_wasm_bindgen`
deserializuje ją bez wymuszania na kliencie `JSON.stringify` i bez definiowania
lustrzanych klas wasm dla czystych DTO. Koszt: brak typu w TS i błąd dopiero w
runtime — w produkcyjnej wersji dodałbym tsify albo przynajmniej walidację z
czytelnym komunikatem (po refaktorze jest: `WasmError::InvalidInput("invalid OOBI array: ...")`).

#### Koncept 5: najpoważniejszy „skeleton in the closet" — `format!("{:?}")` jako format danych

Przed refaktorem (master, lib.rs:59-61 i 252-253):

```rust
pub fn get_kel(&self) -> String {
    format!("{:?}", self.inner.get_own_kel().unwrap())
}
...
let kel = format!("{:?}", signing_identifier.get_own_kel().unwrap());
self.process_kel(kel, None, None)?;
```

**a) Co tu naprawdę się dzieje.** `get_own_kel()` zwraca `Option<Vec<Notice>>`,
a `Notice`/`SignedEventMessage` mają **derived** `Debug` (zweryfikowałem w
źródłach keri-core 0.17.9 — brak custom impl). Więc `get_kel()` zwraca do JS
**Rustowy debug-dump struktur**, nie KEL w CESR. A `process_kel(format!("{:?}"))`
w `incept()`? Zweryfikowałem w źródłach cesrox 0.1.7: `parse_many = many0(parse)`
— `many0` **nigdy nie zawodzi**; na wejściu nie-CESR zwraca `Ok(vec![])`.
Czyli to wywołanie w `incept()` **cicho nie robi nic**, a mimo to „przechodzi".
Verify działa, bo tam KEL przychodzi z watchera jako prawdziwy CESR.

**b) Dlaczego to jest groźne:** (1) `get_kel` jest publicznym API i zwraca
format nieparsowalny maszynowo — konsument nie zrobi z niego backupu/exportu KEL;
(2) `process_kel` nie odróżnia „pusty strumień" od „śmieci na wejściu" — silent
failure mode; (3) mapowanie błędu parsera w `process_kel` jest w praktyce
nieosiągalne dla zwykłych śmieci. Poprawnie: serializować KEL przez
`Notice::to_cesr()` i konkatenować bajty, a w `process_kel` traktować
`parsed.is_empty() && !input.trim().is_empty()` jako błąd.

**c) Pytanie (zadadzą, jeśli poproszą o pokazanie KEL w konsoli):** „Co dokładnie
zwraca `get_kel()` i czy dałoby się to sparsować z powrotem?"
**Modelowa odpowiedź (uczciwa):** Dziś debug-dump — wystarczał do podglądu w
demo, ale to nie jest format wymiany; poprawna wersja emituje CESR przez
`to_cesr()` per event. Wiem o tym, bo audytując kod znalazłem, że `parse_many`
oparte na `many0` maskuje ten problem — i to jest też powód, dla którego
dorzuciłbym test round-trip `get_kel → process_kel` jako pierwszy test modułu.

#### Koncept 6: zarządzanie kluczami — inception i pre-rotacja

```rust
struct KeysConfig { current: SeedPrefix, next: SeedPrefix }
...
let (next_pub_key, _next_secret_keys) = keys.next.derive_key_pair()...;
let next_pub_keys = vec![BasicPrefix::Ed25519NT(next_pub_key)];
let public_keys = vec![BasicPrefix::Ed25519(signer.public_key())];
let signing_inception = self.inner.incept(public_keys, next_pub_keys)...;
...
self.db.add_identifier(&alias, &prefix.clone(), &keys.current)?;
```

**a)** To jest KERI-owa **pre-rotacja**: event inceptujący commit-uje digest
*następnego* klucza publicznego, zanim ten klucz będzie użyty — kompromitacja
bieżącego klucza nie pozwala napastnikowi przejąć rotacji. Dobry materiał, żeby
pokazać rozumienie protokołu, nie tylko Rusta.

**b) Słabe punkty, nazwane wprost:**
1. **`keys.next` jest dropowane na końcu `incept()`** — do DB idzie tylko
   `keys.current`. Commitment na next-key istnieje w KEL, ale sekret przepada:
   **taki identyfikator nigdy nie zrotuje kluczy**. Dopóki binding nie ma
   `rotate()`, to „tylko" dług projektowy — ale przy pytaniu „jak dodasz
   rotację" musisz zacząć od „schemat przechowywania seedów musi objąć next".
2. Seed (`keys.current`) jest zapisywany do IndexedDB **plaintextem**
   (`indexed_db/mod.rs`, store `identifiers`, pole `seed` jako string JSON).
   IndexedDB czyta każdy skrypt z tego samego originu — XSS = exfiltracja
   klucza. Alternatywy: WebCrypto z non-extractable keys (Ed25519 w WebCrypto
   jest już w Safari/Firefox/Chrome), szyfrowanie seedu kluczem z WebCrypto,
   albo przynajmniej passphrase-derived key. To jest najlepsza odpowiedź na
   „co byś zmienił w kwestiach bezpieczeństwa".

**c) Pytanie:** „Gdzie w tym systemie żyje klucz prywatny i jaka jest powierzchnia ataku?"
**Odpowiedź:** Seed generowany z `OsRng` (w wasm: `crypto.getRandomValues` przez
getrandom/js), trzymany w `Arc<Signer>` w pamięci liniowej wasm i utrwalany
plaintextem w IndexedDB. Pamięć wasm nie jest izolowana od JS tego samego
kontekstu — realną granicą jest origin. Wektor: XSS → odczyt IndexedDB.
Mitygacja docelowa: non-extractable CryptoKey + podpis przez WebCrypto, żeby
sekret nigdy nie materializował się w wykradalnej postaci.

#### Koncept 7: obsługa błędów na granicy (stan przed → po refaktorze)

Przed (master): sygnatury `Result<T, JsValue>`, błędy jako `JsValue::from_str(&format!(...))`,
a obok **31 `unwrap/expect/todo!` w samym lib.rs**, w tym na ścieżkach sieciowych:

```rust
let res = Request::get(url.join("introduce").unwrap().as_str())
...
self.signer.sign(add_watcher_event.as_bytes()).unwrap(),
...
cesrox::payload::Payload::MGPK(_items) => todo!(),
```

**a) Czym różni się panika od `Err` w wasm:** panika = `unreachable` trap =
**cała instancja wasm jest martwa** (spójność pamięci niegwarantowana; każdy
kolejny call to ruleta). Nie ma odpowiednika „złap wyjątek i jedź dalej".
Dlatego na granicy FFI unwrap na wejściu/sieci jest kategorycznie gorszy niż
w zwykłym binarce CLI.

**b) Alternatywy dla typu błędu:** (1) `JsValue::from_str` — JS dostaje goły
string (nie `instanceof Error`, bez stacka); (2) `JsError` — prawdziwy JS
`Error`; (3) osobne klasy błędów eksportowane przez wasm-bindgen (enum błędów
jako klasa z polami) — najbogatsze, ale dużo boilerplate'u; (4) **typed enum w
Rust + konwersja na `JsError` na granicy** — wybrane w refaktorze (sekcja 4):
domenowy `thiserror` enum, `From<WasmError> for JsValue`, sygnatury
`Result<T, WasmError>`. JS-owo API bez zmian (dalej „metoda rzuca"), ale
komunikaty są klasyfikowalne i Rust wewnętrznie używa `?`.

**c) Pytanie:** „Dlaczego nie zwracasz po prostu `Result<T, JsError>` wszędzie?"
**Odpowiedź:** `JsError` jest typem granicznym — nie da się na nim matchować w
Rust ani testować bez środowiska JS. Domenowy enum trzyma semantykę
(Network/Cesr/Signing/Database...) testowalną w czystym Rust, a dopiero
`From<WasmError> for JsValue` degraduje ją do wyjątku na samym brzegu.
Jedna konwersja, zero utraty informacji wewnątrz.

---

### 2.3 `src/database/indexed_db/` — warstwa trwałości (1 122 + 766 + 501 + 144 linii)

Architektura (wspólna dla `mod.rs`, `logging.rs`, `sn_database.rs`):

```
write:  API keri-core → HashMap w Rc<RefCell<...>> (sync, natychmiast)
                     → pending_operations.push(...)          (bufor)
persist: setInterval(1000ms) → spawn_local(async flush)      (fire-and-forget)
read:   wyłącznie z HashMap (IndexedDB czytane raz, przy starcie)
```

#### Koncept 8: write-behind cache na jednowątkowym runtime

**a)** IndexedDB ma wyłącznie **asynchroniczne** API, a traity `EventDatabase`/
`LogDatabase` z keri-core są **synchroniczne** (`fn add_kel_finalized_event(&self, ...) -> Result<...>`).
Nie da się „zaczekać" na IndexedDB w synchronicznej metodzie na jednowątkowym
runtime (deadlock z definicji — czekałbyś na event loop, który blokujesz).
Rozwiązanie: cały stan żyje w pamięci (`HashMap`), synchroniczne API operuje na
nim natychmiast, a osobny „wątek logiczny" (`setInterval` + `spawn_local`)
spłukuje bufor operacji do IndexedDB co 1 s. Flaga `Rc<Cell<bool>> flush_in_progress`
to mutex-bez-mutexa: na jednowątkowym runtime `Cell` wystarcza do wykluczenia
współbieżnych flushy (re-entrancy przez event loop, nie przez wątki).

**b) Trade-offy — nazwij je zanim zrobi to rekruter:**
- **Okno utraty danych ≤ ~1 s**: zamknięcie karty między write a flushem gubi
  operacje. Dla event-sourcowanego KEL oznacza to utratę końcówki loga —
  odtwarzalną z watchera, ale to trzeba *powiedzieć*, nie przemilczeć.
- **`Ok(())` kłamie**: metoda raportuje sukces zanim cokolwiek dotknęło dysku;
  błędy flushu są tylko logowane (`log::error!`). Wywołujący nie ma jak się
  dowiedzieć. Alternatywa: `navigator.storage.persist()` + flush na
  `visibilitychange`/`pagehide`, i licznik nieudanych flushy eksponowany w API.
- **Wszystko w RAM**: KEL-e rosną; ta konstrukcja nie strumieniuje z dysku.
  Dla profilu „portfel/edge" OK, dla watchera — nie.
- Alternatywy architektoniczne: (1) crate `idb` (jest w Cargo.toml! nieużywany)
  daje Promise-based API — metody traitów dalej muszą być sync, więc bufor
  zostaje, ale znika cały callback-hell; (2) OPFS + `FileSystemSyncAccessHandle`
  w Web Workerze — jedyne **synchroniczne** storage API w przeglądarce; wymaga
  przeniesienia całego stacku do workera; (3) po prostu wymagać od keri-sdk
  async-traitów — zmiana upstreamu.

**c) Pytanie:** „Dlaczego nie `await`ujesz zapisu do IndexedDB w `add_kel_finalized_event`?"
**Odpowiedź:** Bo sygnatura traitu jest synchroniczna, a runtime jednowątkowy —
blokujące czekanie na wynik async API to deadlock. Wybrałem write-behind z
jawnym buforem; świadomie kupiłem okno utraty ≤1 s i „optymistyczne" `Ok`.
W wersji 2: flush na `pagehide`, `storage.persist()`, i propagacja błędów
flushu przez licznik/callback.

#### Koncept 9: `unsafe impl Send/Sync` — kłamstwo, które trzeba umieć obronić

```rust
// SAFETY: In WebAssembly context, there's no true threading, so these are safe
unsafe impl Send for IndexedDbDatabase {}
unsafe impl Sync for IndexedDbDatabase {}
```

**a)** `Rc`, `RefCell` i uchwyty JS są `!Send`/`!Sync`; keri-sdk wymaga
`Send + Sync` (bo na natywnych targetach działa wielowątkowo). Na
`wasm32-unknown-unknown` bez feature `atomics` nie istnieje drugi wątek mogący
dotknąć tych danych, więc obietnica jest *vacuously true*.

**b)** Ale: to jest obietnica **per-target**, a `unsafe impl` jest bezwarunkowy.
Gdy ktoś skompiluje to z `-C target-feature=+atomics` + wasm-threads (rayon w
wasm istnieje), robi się UB bez ostrzeżenia kompilatora. Minimalny fix:
`#[cfg(target_arch = "wasm32")]` na implach + `compile_error!` przy feature
atomics. Docelowy: zmiana bounds w keri-sdk. To jest wzorcowe pytanie o
rozumienie `unsafe` jako kontraktu, nie mechaniki.

**c) Pytanie (zamknięte):** „Czy `unsafe impl Send` może spowodować UB w
programie, który nigdy nie tworzy wątku?" **Odpowiedź:** Nie bezpośrednio — UB
wymaga faktycznego złamania obietnicy (równoległego dostępu). Ale kod z takim
implem jest *unsound*: bezpieczny kod klienta (np. `std::thread::spawn` po
przeniesieniu na natywny target, wasm-bindgen-rayon) może wywołać UB bez
żadnego `unsafe` u siebie — i to jest definicja unsoundness, którą świadomie
zaakceptowałem i ograniczyłbym `cfg`-iem.

#### Koncept 10: `Closure::wrap` + `.forget()` — callbacki i świadome wycieki

```rust
let success_cb = Closure::wrap(Box::new(move |event: web_sys::Event| { ... }) as Box<dyn FnMut(_)>);
request.set_onsuccess(Some(success_cb.as_ref().unchecked_ref()));
success_cb.forget();
```

**a)** `Closure` to most: Rustowa closura opakowana tak, by JS mógł ją wywołać.
Problem życia: JS może wywołać callback „kiedyś", a Rust chce wiedzieć, kiedy
zwolnić. `.forget()` = celowy leak (closura żyje do końca instancji). Dla
callbacków instalowanych raz (onupgradeneeded/onsuccess przy otwarciu DB,
jeden globalny interval) — akceptowalne i powszechne. Alternatywy: trzymać
`Closure` w polu struktury (drop przy drop struktury), `Closure::once`
(samozwalniająca po pierwszym wywołaniu — pasowałaby do `on_success` w
`AddWatcher`-flush!), FinalizationRegistry.

**b) Słaby punkt:** w `setup_background_flush` (indexed_db/mod.rs:711-767)
closury per-operacja (`on_success` dla każdego `AddWatcher`) też robią
`.forget()` — to leak **proporcjonalny do liczby operacji**, nie stały.
`Closure::once` byłby tu poprawny bez żadnej zmiany architektury.

**c) Pytanie:** „Ile pamięci wycieka ta konstrukcja i czy to problem?"
**Odpowiedź:** Stałe callbacki: O(1), świadomy koszt. Per-operacyjne
`on_success` w watcher-update: O(n) względem liczby update'ów — dług; fix
jednolinijkowy (`Closure::once`). Umiem wskazać różnicę, bo to różne klasy
wycieku.

#### Koncept 11: perełki do samokrytyki w `logging.rs` (nazwane wprost, bo są ciężkie)

1. **Serializacja przez `Debug` i parsowanie stringa z powrotem** —
   `logging.rs:434` zapisuje event tak:

   ```rust
   &JsValue::from_str(&format!("{:?}", serde_cbor::to_vec(&event).unwrap())),
   ```

   czyli CBOR **jako debug-print `Vec<u8>`**: `"[72, 101, 108, ...]"`. A odczyt
   (`logging.rs:565-571`) obcina nawiasy i splituje po `", "`:

   ```rust
   ev_str.remove(0); ev_str.pop();
   for e in ev_str.split(", ") { ev_vec.push(e.parse().unwrap()); }
   ```

   Trzy grzechy naraz: (a) rozdęcie ~4× vs binarnie, (b) `parse().unwrap()` na
   danych z dysku = panika całej instancji na jednym uszkodzonym rekordzie,
   (c) IndexedDB **natywnie przechowuje `Uint8Array`** — wystarczyło
   `Uint8Array::from(bytes.as_slice())`. To jest najsłabszy fragment modułu;
   na rozmowie nazwij go sam („wiem, ten kod ma dług: X, fix to jedna linia").
2. **Niespójność schematu**: zapis kluczuje pola `said`/`event`
   (`logging.rs:430-434`), a odczyt sygnatur szuka pola `digest`
   (`logging.rs:608-618`) — czyli **load sygnatur nigdy nic nie wczyta** z
   danych zapisanych przez własny flush. Do tego flush pisze do store'u
   `"signatures"`, którego `onupgradeneeded` **nie tworzy** (tworzy tylko
   `events`, `trans_receipts`, `nontrans_receipts`) — transakcja na
   nieistniejącym store rzuci błąd, który jest po cichu logowany. Wniosek:
   persystencja sygnatur w ogóle nie działa, a nikt tego nie zauważył — **bo
   nie ma testów**. To najmocniejszy argument „dlaczego testy", jaki masz w
   tym repo, użyj go ofensywnie.

#### Koncept 12: wzorzec „sprawdź-i-unwrapnij" (TOCTOU na transakcjach)

`indexed_db/mod.rs:553-554` (i 5 analogicznych miejsc):

```rust
if !key_state_ops.is_empty() && db.transaction_with_str_and_mode("key_states", ...).is_ok() {
    let transaction = db.transaction_with_str_and_mode("key_states", ...).unwrap();
```

**a)** Tworzy **dwie** transakcje: pierwsza (test `is_ok()`) jest odrzucana,
druga jest używana z `unwrap()` „bo przecież pierwsza się udała". To
nieidiomatyczne (poprawnie: `if let Ok(transaction) = ...`), marnotrawne
(pusta transakcja readwrite) i kruche (druga próba *może* teoretycznie zawieść
inaczej niż pierwsza — np. gdy DB zamknięto pomiędzy). Wystąpienia: linie
553-554, 583-584, 614-615, 643-644, 672-673, 700-701. Ciekawostka: w bloku
`AddWatcher` (719-720) jest już poprawny `if let Ok(...)` — czyli wzorzec
poprawny znałeś, sześć wcześniejszych bloków to kopiuj-wklej starszej wersji.
Dobry przykład na pytanie „co byś zrefaktorował mechanicznie".

#### Koncept 13: trzy instancje `Database` nad jednym magazynem

`lib.rs` (JsController::new):

```rust
let event_database = Arc::new(Database::new());
let tel_database = Arc::new(Database::new());
...
let identifiers_db = Arc::new(Database::new());
```

Każde `Database::new()` otwiera **tę samą** bazę IndexedDB `"keri_indexed_db"`
(plus wewnętrzny `IndexedDbLogDatabase` → `"log_db"`) i ładuje **wszystkie**
store'y do własnych, niezależnych HashMap. Trzy rozjazdowe cache nad wspólną
trwałością: zapis eventu przez `event_database` nie jest widoczny w mapach
`tel_database` do następnego przeładowania strony. W praktyce ratuje cię
rozłączność użycia (event vs tel vs identifiers), ale to jest niezamierzona
własność, nie projekt. Fix: jeden `Arc<Database>` współdzielony w trzech rolach
— `Controller::new(db.clone(), db.clone())`.

---

### 2.4 `src/database/in_memory/` (829 linii) — martwy kod z perspektywy bindingu

`lib.rs` używa wyłącznie `indexed_db` (`use crate::database::indexed_db::IndexedDbDatabase as Database;`).
Moduł `in_memory` jest eksportowany (`pub mod`), ale nic w wasm z niego nie
korzysta — to zaplecze testowo-rozwojowe bez testów, które by go używały.
Architektonicznie ciekawy kontrast do omówienia: tu **nie ma** `unsafe impl` —
używa `RwLock<HashMap>` zamiast `Rc<RefCell>`, więc jest naturalnie `Send+Sync`
(RwLock na wasm działa, bo nigdy nie ma kontencji). To pokazuje, że wariant
„bez unsafe" był możliwy także w indexed_db — cena: `.unwrap()` na lockach
(16 sztuk w `mod.rs`) i brak integracji z JS-owymi callbackami, których RwLock
nie przeżyje (`!Send` closury i tak wymuszą `Rc` przy IndexedDB).

Na rozmowie: nie broń tego modułu jako „feature" — powiedz wprost: powinien być
`#[cfg(test)]`/feature-gated albo wycięty, a jego istnienie bez konsumenta to
szum. (Postać `pub` w opublikowanej bibliotece = zobowiązanie semver za darmo.)

---

## 3. Inwentaryzacja `unwrap()` / `expect()` / `panic!` / `todo!`

Stan **przed refaktorem** (master `d8827a7`): **112 wystąpień** (110 unwrap/expect,
1 `todo!`, 0 `panic!`). Zero w testach, bo testów nie ma.

**Gęstość:** 112 / 3 965 linii raw = **1/35,4**; 112 / 3 381 linii kodu
(niepuste, bez komentarzy) = **1/30,2**; sam `lib.rs`: 31/601 = **1/19,4** —
twoje „~1/20 z audytu" jest trafne dla granicy FFI, zawyżone ~1,7× dla całości.

Klasyfikacja (U = uzasadniony inwariant, N = nieuzasadniony — wejście/IO/sieć,
powinien być `Result`):

### lib.rs (31; po refaktorze: 2)

| Linie (master) | Co | Werdykt |
|---|---|---|
| 60 | `get_own_kel().unwrap()` — Option, None gdy brak KEL | **N** (stan danych) — naprawione |
| 72, 105, 108, 443, 530 | `url.join(...).unwrap()` | N formalnie (na poprawnej bazie join ze stałą ścieżką nie zawiedzie, ale Result jest w sygnaturze) — naprawione przez `?` |
| 95, 245, 424(×2), 498(×2) | `signer.sign(...).unwrap()`, `qry.encode().unwrap()` | **N** (krypto/serializacja zwraca Result) — naprawione |
| 100 | `finalize_add_watcher(...).unwrap()` | **N** (walidacja eventu) — naprawione |
| 117, 121, 402, 403, 440, 446, 494, 527, 533 | `to_cesr().unwrap()`, `Request...body(...).unwrap()`, `serde_json::to_string().unwrap()`, `get_tel_query().unwrap()` | **N** (IO/serializacja) — naprawione |
| 169 | `log::set_logger(...).unwrap()` | **N** — panika przy **drugim** `new JsController()`; naprawione przez `let _ =` |
| 252 | `get_own_kel().unwrap()` w incept | **N** — naprawione |
| 338 | `todo!()` na payload MGPK | **N** — panika na cudzym wejściu; naprawione (`UnsupportedPayload`) |
| 341, 483 | `att.digest.unwrap()` | **N** (pole opcjonalne atestacji) — naprawione (`MissingSaid`) |
| 417, 418, 482 | `watcher_oobi.clone().unwrap()` | **N** (stan konfigurowalny przez użytkownika) — naprawione (`WatcherNotSet`) |
| 485 | `registry_identifier.parse().unwrap()` | **N** (dane z atestacji) — naprawione |
| 575, 578 | `Reflect::set(...).expect(...)` na świeżym obiekcie | **U** — jedyny legalny wyjątek; zostały (z komentarzem SAFETY-like) |

### database/indexed_db/mod.rs (27; po refaktorze: 26)

| Linie | Co | Werdykt |
|---|---|---|
| 105, 1056 | `IndexedDbLogDatabase::new(...)`/`SnDatabase::new(...)` `.unwrap()` | U-ish — te konstruktory dziś nie mogą zawieść (`Ok` zawsze), ale sygnatura Result czyni to kruchym na przyszłość |
| 131, 777 | `window().expect/unwrap` | U w browser-only lib; **N** jeśli deklarujesz wsparcie workerów (w workerze `window()` = None → panika) |
| 455 | `cursor.value().unwrap()` | N (dane z dysku) |
| 554, 584, 615, 644, 673, 701 | drugi `transaction(...).unwrap()` po `is_ok()` | N-idiomatycznie (patrz Koncept 12) |
| 558, 563, 591-593, 620, 624, 649, 653, 680, 683 | `Reflect::set(...).unwrap()` | U (świeży obiekt) — ale w callbacku flushu panika = śmierć instancji; lepiej log |
| 681, 682, 743 | `serde_json::to_string/from_str/to_value(...).unwrap()` na seedzie/scheme | **N** (serializacja danych) |
| 762 | `request.unwrap()` (wynik `store.get`) | **N** (IO) |
| 1032 | `nontrans.unwrap()` na `Option` | **N** — panika gdy brak receiptów dla digestu; **naprawione** (`if let Ok(Some(...))`) |

### database/indexed_db/logging.rs (4; bez zmian)

| Linie | Co | Werdykt |
|---|---|---|
| 51, 759 | `window()` expect/unwrap | U/N jak wyżej |
| 434 | `serde_cbor::to_vec(&event).unwrap()` we flushu | **N** (serializacja; w callbacku → powinno być log-and-skip) |
| 570 | `e.parse().unwrap()` na stringu z dysku | **N — najgorszy pojedynczy unwrap w module**: jeden uszkodzony rekord = trap całej instancji przy load |

### database/indexed_db/sn_database.rs (13; bez zmian)

| Linie | Co | Werdykt |
|---|---|---|
| 62 | `window().expect` | U/N jw. |
| 90, 92, 128, 130, 395, 398 | `event.target().unwrap()`, `dyn_into().unwrap()` w callbackach | U-ish (kontrakt DOM: target istnieje i ma właściwy typ), ale panika w callbacku → lepiej wczesny return + log |
| 201 | `set_interval...expect` | N (IO na window) |
| 239, 258, 273, 292 | `RwLock read/write().unwrap()` | **U** (poisoning tylko po wcześniejszej panice; standard) |
| 433 | `s.as_string().unwrap()` przy konwersji BigInt | N (dane z dysku) |

### database/indexed_db/escrow_database.rs (4; po refaktorze: 0)

67, 83, 127: `digest().unwrap()` — **N** (Result z serializacji), naprawione
przez propagację `IndexedDbError::MissingDigest`; 130: `escrow.remove().unwrap()`
— **N**, trait wymusza `()`, więc naprawione przez log-and-return.

### database/in_memory/ (33; bez zmian — moduł nieużywany przez binding)

- `mod.rs`: 16× locki RwLock (**U**), 2× `digest().unwrap()` (130, 296 — **N**),
  2× konstruktory (49, 103 — U-ish).
- `sn_database.rs`: 4× locki (**U**), 1× konstruktor w `Default` (18 — U-ish).
- `escrow_database.rs`: 5× locki (**U**), 3× `digest().unwrap()` (59, 74, 114 — **N**).

### Podsumowanie klasyfikacji (przed refaktorem)

| Kategoria | Ile | Werdykt |
|---|---|---|
| Locki (RwLock/borrow) | 29 | U — jednowątkowy runtime, poisoning wymaga wcześniejszej paniki |
| `Reflect::set` na świeżych obiektach | 13 | U (2 w lib.rs z expect — wzorcowo; 11 w flushu — U, ale w callbacku wolałbym log) |
| `window()`/DOM-kontrakt/konstruktory | 16 | U-ish (browser-only assumption — zablokuje web workery) |
| Sieć / serializacja / parsowanie / stan / krypto | **53** | **N — powinny być Result** |
| `todo!()` na wejściu użytkownika | 1 | **N** |

Czyli uczciwie: **~48% wystąpień było nieuzasadnionych**, skoncentrowanych
w lib.rs (29/31) i w glue IndexedDB. Po refaktorze (commit `5f54a9a`):
**112 → 78**, w tym lib.rs **31 → 2** (oba uzasadnione), indexed escrow 4 → 0.
Pozostałe 76 to warstwa DB (w większości locki/callbacki — osobny, większy
refaktor; patrz „co dalej" w sekcji 4).

---

## 4. Refaktor — commit `5f54a9a` na branchu `claude/rust-wasm-bindings-prep-xfhf3d`

**Zakres (świadomie ograniczony do granicy FFI + trywialnych propagacji):**

1. Nowy `src/error.rs`: `WasmError` (thiserror) — warianty
   `Url`/`Network`/`Json`/`Cbor`/`Cesr`/`Signing`/`Controller`/`Database`/
   `InvalidInput`/`IdentifierNotFound`/`MissingSaid`/`WatcherNotSet`/
   `UnsupportedPayload`, z `#[from]` dla `url::ParseError`, `gloo_net::Error`,
   `serde_json::Error`, `serde_cbor::Error`, `IndexedDbError` oraz
   `From<WasmError> for JsValue` przez `JsError::new` (JS dostaje prawdziwy
   `Error` z czytelnym message, nie goły string).
2. `lib.rs`: wszystkie publiczne metody zwracają `Result<T, WasmError>`;
   29 nieuzasadnionych unwrapów zastąpione `?`/`map_err`/`ok_or`. Zmiany
   behawioralne (wszystkie na plus, ale wymień je uczciwie):
   - `get_kel()`: `String` → `Result<String, WasmError>` (JS: rzuca zamiast
     ubijać instancję; nadal zwraca debug-dump — naprawa formatu to osobny temat,
     Koncept 5);
   - drugi `new JsController()` już nie panikuje (`log::set_logger` ignorowane
     gdy logger już jest);
   - MGPK: `todo!` → `Err(UnsupportedPayload("MGPK"))`;
   - zepsuty `oobi_array` → `InvalidInput` zamiast **cichego** `unwrap_or(vec![])`
     (null/undefined nadal = pusta lista);
   - błędy sieciowe w `resolve_oobis` są propagowane (wcześniej `let _ =` —
     połykane).
3. `database/indexed_db/escrow_database.rs`: `digest().unwrap()` → propagacja
   `IndexedDbError::MissingDigest`; `remove()` (trait zwraca `()`) →
   log-and-return zamiast paniki.
4. `database/indexed_db/mod.rs:1032`: `Option::unwrap` na receiptach →
   `if let Ok(Some(...))`.

**Nietknięte celowo:** `Ok("")` po wyczerpaniu retry (zmiana semantyki),
serializacja Debug-CBOR w logging.rs (wymaga migracji danych), callback-y DB
(większa przebudowa). To jest dobra odpowiedź na „dlaczego commit jest taki,
a nie większy": refaktor błędów nie powinien przemycać zmian zachowania.

**Weryfikacja kompilacji (toolchain 1.94.1):**

- `cargo check --target wasm32-unknown-unknown` — **OK, 0 warningów** (przed i po).
- `cargo check` (host x86_64) — **również OK**. Różnica, o którą pytałeś:
  na hoście wasm-bindgen/web-sys kompilują się do stubów (typy istnieją,
  importy JS nie są linkowane), więc host-check waliduje typy, ale **nie**
  ABI/link do świata JS. Dlatego job `check` w ci-wasm.yml (bez `--target`)
  przechodzi, ale nie sprawdza tego, co naprawdę pójdzie do wasm-pack —
  warto dodać `--target wasm32-unknown-unknown` do CI (jednolinijkowy PR,
  dobry „dowód aktywności" #2).

**Pełny diff** (738 linii) — commit `5f54a9a`:

<details>
<summary>Rozwiń pełny diff refaktoru</summary>

```diff
diff --git a/bindings/wasm/src/database/indexed_db/escrow_database.rs b/bindings/wasm/src/database/indexed_db/escrow_database.rs
index acf1f3e..97d87d0 100644
--- a/bindings/wasm/src/database/indexed_db/escrow_database.rs
+++ b/bindings/wasm/src/database/indexed_db/escrow_database.rs
@@ -64,7 +64,10 @@ impl EscrowDatabase for IndexedDbEscrowDatabase {
     fn insert(&self, event: &SignedEventMessage) -> Result<(), Self::Error> {
         self.log
             .log_event_with_new_transaction(event)?;
-        let said = event.event_message.digest().unwrap();
+        let said = event
+            .event_message
+            .digest()
+            .map_err(|_| IndexedDbError::MissingDigest)?;
         let id = event.event_message.data.get_prefix();
         let sn = event.event_message.data.sn;
         self.escrow.insert(&id, sn, &said)?;
@@ -80,7 +83,10 @@ impl EscrowDatabase for IndexedDbEscrowDatabase {
     ) -> Result<(), Self::Error> {
         self.log
             .log_event_with_new_transaction(event)?;
-        let said = event.event_message.digest().unwrap();
+        let said = event
+            .event_message
+            .digest()
+            .map_err(|_| IndexedDbError::MissingDigest)?;
 
         self.escrow.insert(id, sn, &said)?;
 
@@ -124,10 +130,20 @@ impl EscrowDatabase for IndexedDbEscrowDatabase {
     }
 
     fn remove(&self, event: &KeriEvent<KeyEvent>) {
-        let said = event.digest().unwrap();
+        // The trait signature does not allow returning an error here, so log
+        // failures instead of panicking.
+        let said = match event.digest() {
+            Ok(said) => said,
+            Err(e) => {
+                log::error!("Failed to compute digest of escrowed event: {}", e);
+                return;
+            }
+        };
         let id = event.data.get_prefix();
         let sn = event.data.sn;
-        self.escrow.remove(&id, sn, &said).unwrap();
+        if let Err(e) = self.escrow.remove(&id, sn, &said) {
+            log::error!("Failed to remove event from escrow: {}", e);
+        }
     }
 
     fn contains(
diff --git a/bindings/wasm/src/database/indexed_db/mod.rs b/bindings/wasm/src/database/indexed_db/mod.rs
index 19e44e6..de47974 100644
--- a/bindings/wasm/src/database/indexed_db/mod.rs
+++ b/bindings/wasm/src/database/indexed_db/mod.rs
@@ -1021,17 +1021,15 @@ impl IndexedDbDatabase {
         for sn in sequence_numbers {
             if let Some(said) = kels_map.get(&(id.to_string(), sn)) {
                 // Get non-transferable couplets for this digest
-                if let Ok(nontrans) = self.log_db.get_nontrans_couplets_by_key(said) {
+                if let Ok(Some(nontrans)) = self.log_db.get_nontrans_couplets_by_key(said) {
                     // Parse identifier
                     if let Ok(identifier) = id.parse::<IdentifierPrefix>() {
                         // Create receipt
                         let rct = Receipt::new(SerializationFormats::JSON, said.clone(), identifier, start);
-                        
+
                         // Create signed receipt with signatures
-                        let signatures = nontrans
-                            .unwrap()
-                            .collect();
-                        
+                        let signatures = nontrans.collect();
+
                         let signed_receipt = SignedNontransferableReceipt {
                             body: rct,
                             signatures,
diff --git a/bindings/wasm/src/error.rs b/bindings/wasm/src/error.rs
new file mode 100644
index 0000000..5701ce2
--- /dev/null
+++ b/bindings/wasm/src/error.rs
@@ -0,0 +1,57 @@
+use wasm_bindgen::{JsError, JsValue};
+
+use crate::database::indexed_db::IndexedDbError;
+
+/// Errors that can cross the wasm boundary.
+///
+/// Every public binding method returns `Result<_, WasmError>`; wasm-bindgen
+/// converts the `Err` variant into a thrown JS `Error` via the `From` impl
+/// below, so JS callers get `instanceof Error` with a readable message
+/// instead of an aborted wasm instance.
+#[derive(Debug, thiserror::Error)]
+pub enum WasmError {
+    #[error("invalid URL: {0}")]
+    Url(#[from] url::ParseError),
+
+    #[error("network request failed: {0}")]
+    Network(#[from] gloo_net::Error),
+
+    #[error("JSON (de)serialization failed: {0}")]
+    Json(#[from] serde_json::Error),
+
+    #[error("CBOR deserialization failed: {0}")]
+    Cbor(#[from] serde_cbor::Error),
+
+    #[error("CESR processing failed: {0}")]
+    Cesr(String),
+
+    #[error("signing failed: {0}")]
+    Signing(String),
+
+    #[error("controller error: {0}")]
+    Controller(String),
+
+    #[error("database error: {0}")]
+    Database(#[from] IndexedDbError),
+
+    #[error("invalid input: {0}")]
+    InvalidInput(String),
+
+    #[error("identifier not found: {0}")]
+    IdentifierNotFound(String),
+
+    #[error("attestation does not contain a SAID digest")]
+    MissingSaid,
+
+    #[error("watcher for identifier is not configured")]
+    WatcherNotSet,
+
+    #[error("{0} payload format is not supported")]
+    UnsupportedPayload(&'static str),
+}
+
+impl From<WasmError> for JsValue {
+    fn from(e: WasmError) -> JsValue {
+        JsError::new(&e.to_string()).into()
+    }
+}
diff --git a/bindings/wasm/src/lib.rs b/bindings/wasm/src/lib.rs
index ed6d5fd..ca538d0 100644
--- a/bindings/wasm/src/lib.rs
+++ b/bindings/wasm/src/lib.rs
@@ -20,7 +20,9 @@ use url::Url;
 use wasm_bindgen::prelude::*;
 
 pub mod database;
+pub mod error;
 use crate::database::indexed_db::IndexedDbDatabase as Database;
+use crate::error::WasmError;
 
 #[wasm_bindgen]
 pub enum VcState {
@@ -56,56 +58,54 @@ impl JsIdentifier {
         self.inner.get_prefix().to_string()
     }
 
-    pub fn get_kel(&self) -> String {
-        format!("{:?}", self.inner.get_own_kel().unwrap())
+    pub fn get_kel(&self) -> Result<String, WasmError> {
+        let kel = self.inner.get_own_kel().ok_or_else(|| {
+            WasmError::IdentifierNotFound(self.inner.get_prefix().to_string())
+        })?;
+        Ok(format!("{:?}", kel))
     }
 
-    pub fn set_alias(&mut self, alias: String) -> Result<(), JsValue> {
-        self.db.update_identifier_alias(&self.alias, &alias).map_err(|e| JsValue::from_str(&format!("Failed to update alias in DB: {}", e)))?;
+    pub fn set_alias(&mut self, alias: String) -> Result<(), WasmError> {
+        self.db.update_identifier_alias(&self.alias, &alias)?;
         self.alias = alias;
         Ok(())
     }
 
-    pub async fn add_watcher(&mut self, url: String) -> Result<(), JsValue> {
-        let url = Url::parse(&url)
-            .map_err(|e| JsValue::from_str(&format!("Invalid URL: {}", e)))?;
-        let res = Request::get(url.join("introduce").unwrap().as_str())
+    pub async fn add_watcher(&mut self, url: String) -> Result<(), WasmError> {
+        let url = Url::parse(&url)?;
+        let res = Request::get(url.join("introduce")?.as_str())
             .send()
-            .await
-            .map_err(|e| JsValue::from_str(&e.to_string()))?;
-        let res_str = res.text().await.map_err(|e| {
-            JsValue::from_str(&format!("Failed to get response: {}", e))
-        })?;
-        let oobi: LocationScheme =
-            serde_json::from_str(&res_str).map_err(|e| {
-                JsValue::from_str(&format!("Failed to parse OOBI: {}", e))
-            })?;
+            .await?;
+        let res_str = res.text().await?;
+        let oobi: LocationScheme = serde_json::from_str(&res_str)?;
         self.watcher_oobi = Some(oobi.clone());
         let watcher_prefix = oobi.clone().eid;
 
         let add_watcher_event = self
             .inner
             .add_watcher(watcher_prefix.clone())
-            .map_err(|e| {
-                JsValue::from_str(&format!("Failed to add watcher: {}", e))
-            })?;
+            .map_err(WasmError::Controller)?;
 
+        let signature = self
+            .signer
+            .sign(add_watcher_event.as_bytes())
+            .map_err(|e| WasmError::Signing(e.to_string()))?;
         let sig = SelfSigningPrefix::new(
             cesrox::primitives::codes::self_signing::SelfSigning::Ed25519Sha512,
-            self.signer.sign(add_watcher_event.as_bytes()).unwrap(),
+            signature,
         );
         let (_, messages) = self
             .inner
             .finalize_add_watcher(add_watcher_event.as_bytes(), sig)
-            .unwrap();
+            .map_err(WasmError::Controller)?;
 
         for message in messages {
             let request_url: Option<String> = match message {
                 Message::Notice(_) => {
-                    Some(url.join("process").unwrap().to_string())
+                    Some(url.join("process")?.to_string())
                 }
                 Message::Op(Op::Reply(_)) => {
-                    Some(url.join("register").unwrap().to_string())
+                    Some(url.join("register")?.to_string())
                 }
                 _ => {
                     log::warn!("Unsupported message type: {:?}", message);
@@ -113,22 +113,19 @@ impl JsIdentifier {
                 }
             };
             if let Some(request_url) = request_url {
-                let body =
-                    Uint8Array::from(message.to_cesr().unwrap().as_slice());
+                let cesr = message
+                    .to_cesr()
+                    .map_err(|e| WasmError::Cesr(e.to_string()))?;
+                let body = Uint8Array::from(cesr.as_slice());
                 let _ = Request::post(&request_url)
                     .header("Content-Type", "application/json")
-                    .body(&body)
-                    .unwrap()
+                    .body(&body)?
                     .send()
-                    .await
-                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
+                    .await?;
             }
         }
 
-        self.db.update_identifier_watcher(
-            &self.alias,
-            oobi.clone(),
-        ).map_err(|e| JsValue::from_str(&format!("Failed to update identifier in DB: {}", e)))?;
+        self.db.update_identifier_watcher(&self.alias, oobi.clone())?;
 
         Ok(())
     }
@@ -165,8 +162,10 @@ impl Default for KeysConfig {
 #[wasm_bindgen]
 impl JsController {
     #[wasm_bindgen(constructor)]
-    pub fn new() -> Result<JsController, JsValue> {
-        log::set_logger(&wasm_bindgen_console_logger::DEFAULT_LOGGER).unwrap();
+    pub fn new() -> Result<JsController, WasmError> {
+        // Setting the logger fails when one is already installed (e.g. a
+        // second controller is constructed) — keep the existing logger then.
+        let _ = log::set_logger(&wasm_bindgen_console_logger::DEFAULT_LOGGER);
         log::set_max_level(log::LevelFilter::Info);
 
         let event_database = Arc::new(Database::new());
@@ -184,23 +183,23 @@ impl JsController {
     pub fn load_identifier(
         &self,
         alias: String,
-    ) -> Result<JsIdentifier, JsValue> {
+    ) -> Result<JsIdentifier, WasmError> {
         let id_record = self
             .db
             .get_identifier(&alias)
-            .ok_or_else(|| JsValue::from_str("Identifier not found"))?;
+            .ok_or_else(|| WasmError::IdentifierNotFound(alias.clone()))?;
 
         let identifier = self
             .inner
             .load_identifier(&id_record.said)
-            .map_err(|e| JsValue::from_str(&format!("Load identifier error: {}", e)))?;
+            .map_err(WasmError::Controller)?;
         let signer = Signer::new_with_seed(&id_record.seed)
-            .map_err(|e| JsValue::from_str(&format!("Signer creation error: {}", e)))?;
+            .map_err(|e| WasmError::Signing(e.to_string()))?;
 
         Ok(JsIdentifier::new(alias, identifier, Arc::new(signer), self.db.clone(), id_record.watcher_oobi))
     }
 
-    pub fn get_identifier_aliases(&self) -> Result<Vec<JsValue>, JsValue> {
+    pub fn get_identifier_aliases(&self) -> Result<Vec<JsValue>, WasmError> {
         let aliases: Vec<JsValue> = self
             .db
             .get_identifiers()
@@ -210,53 +209,50 @@ impl JsController {
         Ok(aliases)
     }
 
-    pub fn incept(&self) -> Result<JsIdentifier, JsValue> {
+    pub fn incept(&self) -> Result<JsIdentifier, WasmError> {
         let keys = KeysConfig::default();
         let (next_pub_key, _next_secret_keys) =
-            match keys.next.derive_key_pair() {
-                Ok(pair) => pair,
-                Err(e) => {
-                    return Err(JsValue::from_str(&format!(
-                        "Failed to derive keys: {}",
-                        e
-                    )))
-                }
-            };
+            keys.next.derive_key_pair().map_err(|e| {
+                WasmError::Signing(format!("failed to derive keys: {}", e))
+            })?;
 
-        let signer = match Signer::new_with_seed(&keys.current.clone()) {
-            Ok(s) => Arc::new(s),
-            Err(e) => {
-                return Err(JsValue::from_str(&format!(
-                    "Failed to create signer: {}",
-                    e
-                )))
-            }
-        };
+        let signer = Signer::new_with_seed(&keys.current)
+            .map(Arc::new)
+            .map_err(|e| WasmError::Signing(e.to_string()))?;
 
         let next_pub_keys = vec![BasicPrefix::Ed25519NT(next_pub_key)];
         let public_keys = vec![BasicPrefix::Ed25519(signer.public_key())];
 
         let signing_inception =
-            self.inner
-                .incept(public_keys, next_pub_keys)
-                .map_err(|_| JsValue::from_str("Incept error"))?;
+            self.inner.incept(public_keys, next_pub_keys).map_err(|()| {
+                WasmError::Controller(
+                    "inception event generation failed".to_string(),
+                )
+            })?;
+        let signature = signer
+            .sign(signing_inception.as_bytes())
+            .map_err(|e| WasmError::Signing(e.to_string()))?;
         let signature = SelfSigningPrefix::new(
             cesrox::primitives::codes::self_signing::SelfSigning::Ed25519Sha512,
-            signer.sign(signing_inception.as_bytes()).unwrap(),
+            signature,
         );
         let signing_identifier = self
             .inner
             .finalize_incept(signing_inception.as_bytes(), &signature)
-            .map_err(|_| JsValue::from_str("Finalize error"))?;
+            .map_err(|()| {
+                WasmError::Controller("inception finalization failed".to_string())
+            })?;
 
-        let kel = format!("{:?}", signing_identifier.get_own_kel().unwrap());
-        self.process_kel(kel, None, None)?;
+        let kel = signing_identifier.get_own_kel().ok_or_else(|| {
+            WasmError::IdentifierNotFound(
+                signing_identifier.get_prefix().to_string(),
+            )
+        })?;
+        self.process_kel(format!("{:?}", kel), None, None)?;
 
         let prefix = signing_identifier.get_prefix();
         let alias = prefix.to_string();
-        self.db.add_identifier(&alias, &prefix.clone(), &keys.current).map_err(|e| {
-            JsValue::from_str(&format!("Failed to add identifier to DB: {}", e))
-        })?;
+        self.db.add_identifier(&alias, &prefix.clone(), &keys.current)?;
 
         Ok(JsIdentifier::new(alias, signing_identifier, signer.clone(), self.db.clone(), None))
     }
@@ -266,10 +262,10 @@ impl JsController {
         kel: String,
         from: Option<u64>,
         limit: Option<u64>,
-    ) -> Result<(), JsValue> {
+    ) -> Result<(), WasmError> {
         let mut parsed_kel: Vec<Message> = parse_event_stream(kel.as_bytes())
             .map_err(|e| {
-                JsValue::from_str(&format!("Failed to parse KEL: {}", e))
+                WasmError::Cesr(format!("failed to parse KEL: {}", e))
             })?;
         if let Some(from) = from {
             parsed_kel = parsed_kel
@@ -284,35 +280,35 @@ impl JsController {
                 .collect();
         }
 
-        self.inner.process_kel(&parsed_kel).map_err(|e| {
-            JsValue::from_str(&format!("Process events error: {}", e))
-        })?;
+        self.inner
+            .process_kel(&parsed_kel)
+            .map_err(WasmError::Controller)?;
 
         Ok(())
     }
 
-    pub fn process_tel(&self, tel: String) -> Result<(), JsValue> {
-        self.inner.process_tel(tel.as_bytes()).map_err(|e| {
-            JsValue::from_str(&format!("Process events error: {}", e))
-        })?;
+    pub fn process_tel(&self, tel: String) -> Result<(), WasmError> {
+        self.inner
+            .process_tel(tel.as_bytes())
+            .map_err(WasmError::Controller)?;
 
         Ok(())
     }
 
-    pub fn get_vc_state(&self, prefix: String) -> Result<VcState, JsValue> {
+    pub fn get_vc_state(&self, prefix: String) -> Result<VcState, WasmError> {
         let said: SelfAddressingIdentifier = prefix.parse().map_err(|e| {
-            JsValue::from_str(&format!("Invalid prefix: {}", e))
+            WasmError::InvalidInput(format!("invalid prefix: {}", e))
         })?;
 
-        self.inner.get_vc_state(&said)
-            .map_err(|e| JsValue::from_str(&format!("Get VC state error: {}", e)))
-            .map(|state| {
-                match state {
-                    Some(TelState::Issued(_)) => VcState::Issued,
-                    Some(TelState::Revoked) => VcState::Revoked,
-                    None | Some(TelState::NotIssued) => VcState::NotIssued,
-                }
-            })
+        let state = self
+            .inner
+            .get_vc_state(&said)
+            .map_err(WasmError::Controller)?;
+        Ok(match state {
+            Some(TelState::Issued(_)) => VcState::Issued,
+            Some(TelState::Revoked) => VcState::Revoked,
+            None | Some(TelState::NotIssued) => VcState::NotIssued,
+        })
     }
 
     pub async fn verify(
@@ -320,69 +316,59 @@ impl JsController {
         identifier: &JsIdentifier,
         oobi_array: JsValue,
         message: String,
-    ) -> Result<JsValue, JsValue> {
+    ) -> Result<JsValue, WasmError> {
         let (_rest, cesr) = cesrox::parse(message.as_bytes()).map_err(|e| {
-            JsValue::from_str(&format!("Failed to parse CESR: {}", e))
+            WasmError::Cesr(format!("failed to parse CESR: {}", e))
         })?;
         let att: acdc::Attestation = match cesr.payload {
             cesrox::payload::Payload::JSON(items) => {
-                serde_json::from_slice(&items).map_err(|_e| ()).map_err(
-                    |_| JsValue::from_str("Failed to parse JSON payload"),
-                )?
+                serde_json::from_slice(&items)?
             }
             cesrox::payload::Payload::CBOR(items) => {
-                serde_cbor::from_slice(&items).map_err(|_e| ()).map_err(
-                    |_| JsValue::from_str("Failed to parse CBOR payload"),
-                )?
+                serde_cbor::from_slice(&items)?
+            }
+            cesrox::payload::Payload::MGPK(_items) => {
+                return Err(WasmError::UnsupportedPayload("MGPK"))
             }
-            cesrox::payload::Payload::MGPK(_items) => todo!(),
         };
 
-        let vc_said = att.clone().digest.unwrap();
+        let vc_said = att.digest.clone().ok_or(WasmError::MissingSaid)?;
         let current_vc_state = self.get_vc_state(vc_said.to_string())?;
         if let VcState::Revoked = current_vc_state {
             let result: VerificationResult = VcState::Revoked.into();
             return Ok(result.into());
         }
 
-        let oobis: Vec<Oobi> =
-            serde_wasm_bindgen::from_value(oobi_array).unwrap_or(vec![]);
+        let oobis: Vec<Oobi> = if oobi_array.is_null()
+            || oobi_array.is_undefined()
+        {
+            vec![]
+        } else {
+            serde_wasm_bindgen::from_value(oobi_array).map_err(|e| {
+                WasmError::InvalidInput(format!("invalid OOBI array: {}", e))
+            })?
+        };
         let watcher_url = identifier
             .watcher_oobi
             .clone()
-            .ok_or_else(|| JsValue::from_str("Watcher for identifier not set"))?
+            .ok_or(WasmError::WatcherNotSet)?
             .url;
         self.resolve_oobis(&watcher_url.to_string(), oobis.clone())
-            .await
-            .map_err(|e| {
-                JsValue::from_str(&format!("Failed to resolve OOBIs: {:?}", e))
-            })?;
+            .await?;
 
         let issuer_id: IdentifierPrefix = att.issuer.parse().map_err(|e| {
-            JsValue::from_str(&format!("Failed to parse issuer ID: {}", e))
+            WasmError::InvalidInput(format!("failed to parse issuer ID: {}", e))
         })?;
         let current_state = self
             .inner
             .get_state(&issuer_id);
         let current_sn = current_state.clone().map(|s| s.sn);
-        let kel =
-            self.query_kel(identifier, issuer_id, current_sn).await.map_err(|e| {
-                JsValue::from_str(&format!("Failed to query KEL: {:?}", e))
-            })?;
+        let kel = self.query_kel(identifier, issuer_id, current_sn).await?;
         let skip_first = current_state.as_ref().map(|_| 1);
-        self.process_kel(kel, skip_first, None).map_err(|e| {
-            JsValue::from_str(&format!("Failed to process KEL: {:?}", e))
-        })?;
+        self.process_kel(kel, skip_first, None)?;
 
-        let tel =
-            self.query_tel(identifier, att.clone())
-                .await
-                .map_err(|e| {
-                    JsValue::from_str(&format!("Failed to query TEL: {:?}", e))
-                })?;
-        self.process_tel(tel).map_err(|e| {
-            JsValue::from_str(&format!("Failed to process TEL: {:?}", e))
-        })?;
+        let tel = self.query_tel(identifier, att.clone()).await?;
+        self.process_tel(tel)?;
 
         let vc_state = self.get_vc_state(vc_said.to_string())?;
         let result: VerificationResult = vc_state.into();
@@ -393,17 +379,15 @@ impl JsController {
 impl JsController {
     async fn resolve_oobis(
         &self,
-        watcher_url: &String,
+        watcher_url: &str,
         oobis: Vec<Oobi>,
-    ) -> Result<(), JsValue> {
+    ) -> Result<(), WasmError> {
         for oobi in oobis {
-            let _ = Request::post(&format!("{}resolve", watcher_url))
+            Request::post(&format!("{}resolve", watcher_url))
                 .header("Content-Type", "application/json")
-                .body(serde_json::to_string(&oobi).unwrap())
-                .unwrap()
+                .body(serde_json::to_string(&oobi)?)?
                 .send()
-                .await
-                .map_err(|e| JsValue::from_str(&e.to_string()));
+                .await?;
         }
         Ok(())
     }
@@ -413,15 +397,25 @@ impl JsController {
         signing_id: &JsIdentifier,
         id: IdentifierPrefix,
         from_sn: Option<u64>,
-    ) -> Result<String, JsValue> {
-        let watcher_url = signing_id.watcher_oobi.clone().unwrap().url;
-        let watcher_id = signing_id.watcher_oobi.clone().unwrap().eid;
+    ) -> Result<String, WasmError> {
+        let watcher_oobi = signing_id
+            .watcher_oobi
+            .clone()
+            .ok_or(WasmError::WatcherNotSet)?;
+        let watcher_url = watcher_oobi.url;
+        let watcher_id = watcher_oobi.eid;
         let qry = signing_id.inner.get_log_query(id, watcher_id, from_sn, None);
         let signer = signing_id.signer.clone();
 
+        let encoded_qry = qry
+            .encode()
+            .map_err(|e| WasmError::Cesr(e.to_string()))?;
+        let signature = signer
+            .sign(encoded_qry)
+            .map_err(|e| WasmError::Signing(e.to_string()))?;
         let sig = SelfSigningPrefix::new(
             cesrox::primitives::codes::self_signing::SelfSigning::Ed25519Sha512,
-            signer.sign(qry.encode().unwrap()).unwrap(),
+            signature,
         );
         let signatures = vec![IndexedSignature::new_both_same(sig, 0)];
         let singed_kel_qry = SignedKelQuery::new_trans(
@@ -436,31 +430,19 @@ impl JsController {
             let signed_qry =
                 SignedQueryMessage::KelQuery(singed_kel_qry.clone());
 
-            let body_msg =
-                Message::Op(Op::Query(signed_qry)).to_cesr().unwrap();
+            let body_msg = Message::Op(Op::Query(signed_qry))
+                .to_cesr()
+                .map_err(|e| WasmError::Cesr(e.to_string()))?;
             let body = js_sys::Uint8Array::from(body_msg.as_slice());
-            let response =
-                Request::post(watcher_url.join("query").unwrap().as_str())
-                    .header("Content-Type", "application/json")
-                    .body(&body)
-                    .unwrap()
-                    .send()
-                    .await
-                    .map_err(|e| {
-                        JsValue::from_str(&format!(
-                            "Failed to send request: {}",
-                            e
-                        ))
-                    })?;
+            let response = Request::post(watcher_url.join("query")?.as_str())
+                .header("Content-Type", "application/json")
+                .body(&body)?
+                .send()
+                .await?;
 
             let code = response.status();
             if code == 200 {
-                kel = response.text().await.map_err(|e| {
-                    JsValue::from_str(&format!(
-                        "Failed to get response text: {}",
-                        e
-                    ))
-                })?;
+                kel = response.text().await?;
                 break;
             } else {
                 gloo_timers::future::TimeoutFuture::new(
@@ -478,11 +460,22 @@ impl JsController {
         &self,
         id: &JsIdentifier,
         acdc_attestation: acdc::Attestation,
-    ) -> Result<String, JsValue> {
-        let watcher_url = id.watcher_oobi.clone().unwrap().url;
-        let vc_said = acdc_attestation.digest.unwrap();
-        let registry_id: said::SelfAddressingIdentifier =
-            acdc_attestation.registry_identifier.parse().unwrap();
+    ) -> Result<String, WasmError> {
+        let watcher_url = id
+            .watcher_oobi
+            .clone()
+            .ok_or(WasmError::WatcherNotSet)?
+            .url;
+        let vc_said = acdc_attestation.digest.ok_or(WasmError::MissingSaid)?;
+        let registry_id: said::SelfAddressingIdentifier = acdc_attestation
+            .registry_identifier
+            .parse()
+            .map_err(|e| {
+                WasmError::InvalidInput(format!(
+                    "invalid registry identifier: {}",
+                    e
+                ))
+            })?;
         let signer = id.signer.clone();
 
         let tel_qry = id
@@ -491,11 +484,17 @@ impl JsController {
                 IdentifierPrefix::SelfAddressing(registry_id.into()),
                 IdentifierPrefix::SelfAddressing(vc_said.clone().into()),
             )
-            .unwrap();
-
+            .map_err(WasmError::Controller)?;
+
+        let encoded_qry = tel_qry
+            .encode()
+            .map_err(|e| WasmError::Cesr(e.to_string()))?;
+        let signature = signer
+            .sign(encoded_qry)
+            .map_err(|e| WasmError::Signing(e.to_string()))?;
         let signature_tel_query = SelfSigningPrefix::new(
             cesrox::primitives::codes::self_signing::SelfSigning::Ed25519Sha512,
-            signer.sign(tel_qry.encode().unwrap()).unwrap(),
+            signature,
         );
 
         let tel_query = match &id.inner.id {
@@ -523,31 +522,20 @@ impl JsController {
         let mut delay = std::time::Duration::from_secs(1);
         let mut tel = "".to_string();
         for _i in 0..5 {
-            let body = js_sys::Uint8Array::from(
-                tel_query.to_cesr().unwrap().as_slice(),
-            );
+            let body_msg = tel_query
+                .to_cesr()
+                .map_err(|e| WasmError::Cesr(e.to_string()))?;
+            let body = js_sys::Uint8Array::from(body_msg.as_slice());
             let response =
-                Request::post(watcher_url.join("query/tel").unwrap().as_str())
+                Request::post(watcher_url.join("query/tel")?.as_str())
                     .header("Content-Type", "application/json")
-                    .body(&body)
-                    .unwrap()
+                    .body(&body)?
                     .send()
-                    .await
-                    .map_err(|e| {
-                        JsValue::from_str(&format!(
-                            "Failed to send request: {}",
-                            e
-                        ))
-                    })?;
+                    .await?;
 
             let code = response.status();
             if code == 200 {
-                tel = response.text().await.map_err(|e| {
-                    JsValue::from_str(&format!(
-                        "Failed to get response text: {}",
-                        e
-                    ))
-                })?;
+                tel = response.text().await?;
                 break;
             } else {
                 gloo_timers::future::TimeoutFuture::new(
@@ -571,6 +559,8 @@ impl From<VerificationResult> for JsValue {
     fn from(val: VerificationResult) -> Self {
         let obj = js_sys::Object::new();
 
+        // Reflect::set on a freshly created object cannot fail — these
+        // expects guard an invariant, not fallible input.
         js_sys::Reflect::set(&obj, &JsValue::from_str("verified"), &JsValue::from_bool(val.verified))
             .expect("setting verified failed");
 
```

</details>

---

## 5. WASM (dkms-bindings) vs NAPI (oca-conductor) — „kiedy które"

*Zastrzeżenie: w tej sesji nie mogłem sklonować `THCLab/oca-conductor`
(ograniczenie środowiska: sesja jest zescope'owana do repozytoriów właściciela
`olichwiruk`, a dodanie repo innej organizacji zostało odrzucone). Porównanie
opiera się na architekturze napi-rs i publicznej strukturze oca-conductor
(rdzeń walidacji OCA + `bindings/node.js` na napi-rs), nie na świeżym odczycie
kodu — zweryfikuj szczegóły przed rozmową.*

W NAPI natywna biblioteka (`.node`, cdylib) jest ładowana do procesu Node,
więc obiekty Rust żyją na natywnej stercie i są opakowywane w obiekty JS z
finalizerami — GC Node'a domyka ich życie, podczas gdy w wasm-bindgen JS trzyma
goły indeks do pamięci liniowej i to ty (lub FinalizationRegistry) odpowiadasz
za `free()`. NAPI daje prawdziwe wątki: możesz trzymać pulę tokio/rayon i
zwracać Promise przez `AsyncTask`/`ThreadsafeFunction`, więc ciężka walidacja
schodzi z pętli zdarzeń; w wasm wszystko dzieli jeden wątek z UI, a „async" to
tylko przeplatanie na event loopie (stąd cały cyrk write-behind z sekcji 2.3 —
w NAPI po prostu użyłbyś blokującego I/O na wątku roboczym). NAPI ma pełny
dostęp do OS (pliki, env, gniazda) — oca-conductor może czytać bundlery OCA
z dysku; wasm siedzi w piaskownicy i każde I/O pożycza od JS (fetch, IndexedDB),
co widzisz w tym module jako największy generator złożoności. Dystrybucja jest
odwrotnością: napi-rs wymaga matrixa prebuiltów per platforma×arch×libc
(np. przez CI i `@org/pkg-linux-x64-gnu`), wasm to **jeden** artefakt działający
w przeglądarce, Node ≥ 8-ish, Deno i na edge'u. Wydajnościowo NAPI wygrywa
(natywny kod, SIMD, wątki, zero kopii przez external buffers), wasm płaci
kopiowaniem na granicy i brakiem wątków, ale bywa „wystarczająco szybki".
Reguła decyzyjna, którą możesz powiedzieć jednym zdaniem: **jeśli kod musi
działać w przeglądarce (jak portfel DKMS trzymający klucze przy użytkowniku),
wasm jest jedyną opcją; jeśli celem jest wyłącznie serwerowy Node i liczy się
przepustowość, wątki lub dostęp do OS (jak walidator OCA w pipeline), NAPI jest
lepszym narzędziem; przy obu targetach naraz — wasm jako wspólny mianownik,
NAPI jako akcelerator tam, gdzie profiler każe.*

---

## 6. Bank pytań rekrutacyjnych (wygenerowane z konkretnych linii tego repo)

Zamknięte = da się odpowiedzieć w 1-2 zdaniach; otwarte = dyskusja projektowa.
Przy każdym: kotwica w kodzie.

1. **[zamknięte]** `indexed_db/mod.rs:86-87` — `unsafe impl Send for IndexedDbDatabase`.
   Jaki dokładnie kontrakt łamiesz i co musiałoby się stać, żeby to było UB?
   *(oczekiwane: kontrakt „można dotykać z wielu wątków"; UB przy realnych
   wątkach wasm/atomics lub przeniesieniu na natywny target; fix: cfg per target)*
2. **[zamknięte]** `lib.rs` (master:169) — dlaczego `new JsController()` wywołane
   drugi raz panikowało? *(log::set_logger zwraca Err przy ponownej instalacji;
   unwrap; w wasm panika = martwa instancja)*
3. **[otwarte]** `examples/web/main.js` ma `setTimeout(..., 2000)` po
   `new JsController()`. Jaki race ukrywa ten hack i jak zaprojektowałbyś API,
   żeby go nie było? *(konstruktor sync, load z IndexedDB async fire-and-forget;
   poprawnie: async fabryka `JsController.create()` zwracająca Promise po
   zakończeniu load, albo jawne `await controller.ready()`)*
4. **[zamknięte]** `Cargo.toml:39` — po co `wasm-bindgen = "=0.2.92"` z `=`?
   *(ABI crate↔CLI musi być identyczne; CI instaluje cli 0.2.92)*
5. **[otwarte]** `indexed_db/mod.rs` — write-behind z `setInterval(1000)`:
   jakie gwarancje trwałości dajesz wywołującemu `add_kel_finalized_event`,
   który dostał `Ok(())`? Co z zamknięciem karty? *(≤1 s okno utraty; flush na
   pagehide/visibilitychange; storage.persist(); propagacja błędów flushu)*
6. **[zamknięte]** `logging.rs:434` + `565-571` — co jest nie tak z zapisem
   `format!("{:?}", serde_cbor::to_vec(...))` i odczytem przez `split(", ")`?
   *(debug-print bajtów jako format danych; 4× rozdęcie; parse().unwrap() na
   danych z dysku = trap; IndexedDB umie Uint8Array natywnie)*
7. **[zamknięte]** `logging.rs` — flush pisze store `"signatures"`, którego
   `onupgradeneeded` nie tworzy, a load sygnatur czyta pole `digest`, gdy zapis
   kluczuje `said`. Jak to możliwe, że „działa"? *(nie działa — błędy są cicho
   logowane, odczyt ładuje 0 rekordów, a stan i tak żyje w RAM; wykryłby to
   pierwszy test round-trip persist→load)*
8. **[otwarte]** `lib.rs` (master:59-61) — `get_kel()` zwraca
   `format!("{:?}", ...)`. Co dostaje JS, dlaczego to przechodzi przez
   `process_kel` bez błędu i jak wygląda poprawna wersja? *(debug-dump;
   cesrox parse_many = many0 → Ok(vec![]) na śmieciach — silent no-op;
   poprawnie: to_cesr() per Notice + traktowanie „0 sparsowanych z niepustego
   wejścia" jako błąd)*
9. **[zamknięte]** `lib.rs` (master:349) — `serde_wasm_bindgen::from_value(oobi_array).unwrap_or(vec![])`.
   Czym różni się to od wersji po refaktorze i które zachowanie jest poprawne
   dla API publicznego? *(cicho ignoruje zepsute wejście vs InvalidInput;
   null/undefined jako „brak" jest OK, śmieci nie)*
10. **[otwarte]** `lib.rs` incept() — gdzie ląduje `keys.next` i co to oznacza
    dla rotacji kluczy tego identyfikatora? *(dropowane; commitment w KEL jest,
    sekretu nie ma — AID nierotowalne, dopóki schemat storage nie obejmie next;
    dobra okazja do wyjaśnienia pre-rotacji KERI)*
11. **[otwarte]** Seed w IndexedDB plaintextem (`indexed_db/mod.rs`, store
    `identifiers`). Model zagrożeń i mitygacje? *(XSS = exfiltracja; WebCrypto
    non-extractable, szyfrowanie at-rest, separacja originu, ewentualnie
    signing w workerze)*
12. **[zamknięte]** `query_kel` (lib.rs) — po 5 próbach zwraca `Ok("")`. Jaki
    jest efekt końcowy w `verify` i jak to naprawić? *(werdykt na stanie sprzed
    query — potencjalnie stale; Err(WatcherUnavailable) po wyczerpaniu prób)*
13. **[zamknięte]** `indexed_db/mod.rs:553-554` — dwa wywołania
    `transaction_with_str_and_mode` pod rząd. Co tu jest nieidiomatyczne i jak
    to zapisać poprawnie? *(check-then-unwrap zamiast `if let Ok`; druga
    transakcja może zawieść niezależnie; pusta transakcja readwrite w koszcie)*
14. **[otwarte]** `Closure::wrap` + `.forget()` w trzech plikach DB — kiedy ten
    leak jest O(1), a kiedy O(n), i gdzie w tym kodzie jest wariant O(n)?
    *(instalowane raz vs per-operacja; on_success w AddWatcher-flush
    (mod.rs:726nn) leakuje per update; Closure::once)*
15. **[zamknięte]** Dlaczego `in_memory` używa `RwLock`, a `indexed_db`
    `Rc<RefCell>` — i który wariant wymusił `unsafe impl Send/Sync`?
    *(RwLock jest Send/Sync naturalnie, ale nie przeżyje !Send closur JS;
    Rc/RefCell integruje się z callbackami, za to wymaga unsafe deklaracji
    wobec bounds keri-sdk)*
16. **[otwarte]** Trait `EventDatabase` jest synchroniczny, IndexedDB tylko
    async. Omów przestrzeń rozwiązań i uzasadnij wybraną. *(write-behind
    cache — wybrane; idb crate dla czytelności; OPFS sync handle w workerze;
    async-trait w SDK; każde z kosztami)*
17. **[zamknięte]** `profile.release`: `opt-level="z"`, `lto=true`, `strip=true`
    — po co każda z tych trzech rzeczy w kontekście dystrybucji wasm i co robią
    flagi `--enable-bulk-memory --enable-nontrapping-float-to-int` w wasm-opt?
    *(rozmiar downloadu; post-MVP featury emitowane przez LLVM, które starszy
    wasm-opt musi mieć włączone jawnie)*
18. **[zamknięte]** CI: job `check` robi `cargo check` bez `--target
    wasm32-unknown-unknown` i przechodzi. Co realnie sprawdza, a czego nie?
    *(typy na hoście ze stubami wasm-bindgen; nie sprawdza targetu
    produkcyjnego ani glue; fix jednolinijkowy)*
19. **[otwarte]** W module nie ma ani jednego testu. Od czego byś zaczął, mając
    jeden dzień? *(modelowa odpowiedź: (1) wasm-bindgen-test round-trip
    persist→load IndexedDB — od razu wykrywa bugi z pytania 7; (2) property
    test get_kel→process_kel po naprawie formatu; (3) test kontraktu błędów:
    zepsute OOBI/MGPK/brak watchera rzucają, nie trapują; CI: headless chrome)*
20. **[otwarte]** `JsController::new` tworzy trzy `Database::new()` nad tą samą
    bazą IndexedDB. Jakie są konsekwencje i jak wygląda minimalna poprawka?
    *(trzy niezależne cache nad wspólnym storage — rozjazd do przeładowania
    strony; jeden Arc<Database> w trzech rolach)*

---

*Wygenerowano na potrzeby przygotowania do rozmów; refaktor: commit `5f54a9a`,
branch `claude/rust-wasm-bindings-prep-xfhf3d`.*
