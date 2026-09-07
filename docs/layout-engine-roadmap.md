# Layout Engine Roadmap

## Cel ogólny zmiany

Celem tej zmiany jest przebudowanie mechanizmu przełączania layoutów z modelu:

- "wykonaj listę kruchych kroków i licz, że zadziała"

na model:

- "sprawdź możliwość wdrożenia, zaplanuj wykonanie, zastosuj bezpiecznie, zweryfikuj wynik i w razie potrzeby wycofaj zmiany"

Praktycznie ma to dać:

- większą odporność na aktualizacje GNOME i różnice między systemami
- lepszą diagnostykę błędów
- możliwość wdrożenia trybu `degraded` zamiast twardych awarii
- fundament pod późniejszy pivot do deklaratywnej specyfikacji UX i adapterów backendowych

## Plan zmiany

### 1. Wprowadzić formalny wynik operacji zastosowania layoutu

Cel:

- odejść od `Result<()>` jako jedynego kontraktu
- rozróżniać `success`, `degraded`, `blocked`, `rolled_back`, `failed_partial`

Pliki:

- `crates/domain/src/layout.rs`
- `crates/app/src/layout_service.rs`
- opcjonalnie `crates/app/src/progress.rs`

Efekt:

- UI i historia będą wiedziały, co naprawdę się stało, a nie tylko "OK / error"

### 2. Dodać etap `preflight`

Cel:

- przed modyfikacją systemu sprawdzić, czy layout ma sens dla aktualnego środowiska

Zakres sprawdzeń:

- aktywne DE i jego wersja
- zgodność z `min_version`
- dostępność narzędzi (`gext`, `dconf`, `gsettings`, `busctl`)
- możliwość instalacji rozszerzeń
- zgodność systemu i package managera
- wykrycie brakujących zależności

Pliki:

- `crates/app/src/layout_service.rs`
- `crates/infra/src/platform.rs`
- `crates/infra/src/tools.rs`
- `crates/infra/src/extensions.rs`
- `crates/domain/src/layout.rs`

Efekt:

- zanim zaczniemy cokolwiek zmieniać, będziemy wiedzieć, czy layout jest `supported`, `degraded` czy `blocked`

### 3. Wydzielić `ExecutionPlan`

Cel:

- oddzielić planowanie od wykonania

Plan powinien zawierać:

- listę kroków do uruchomienia
- kroki pominięte
- konflikty
- wykryte ryzyka
- przewidywany poziom degradacji
- ewentualne akcje rollback

Pliki:

- nowy plik `crates/app/src/execution_plan.rs`
- `crates/app/src/layout_service.rs`
- `crates/app/src/lib.rs`

Efekt:

- silnik stanie się przewidywalny i testowalny, a nie monolityczny

### 4. Naprawić semantykę błędów podczas `apply`

Cel:

- obecnie pojedyncze błędy kroków są logowane, ale proces często idzie dalej
- trzeba rozróżnić błędy krytyczne i miękkie

Zmiana:

- krok może być `fatal` albo `soft`
- błąd `fatal` zatrzymuje proces i uruchamia rollback
- błąd `soft` obniża wynik do `degraded`
- verification failure też ma jasną politykę

Pliki:

- `crates/domain/src/layout.rs`
- `crates/app/src/layout_service.rs`

Efekt:

- koniec z częściowo zastosowanymi layoutami udającymi sukces

### 5. Rozszerzyć model `.de` o metadane kompatybilności

Cel:

- przestać traktować layout jako samą listę kroków

Nowe pola do rozważenia:

- `spec_version`
- `provider`
- `variant`
- `capabilities_required`
- `capabilities_optional`
- `degraded_behavior`
- `rollback_policy`
- `preflight_checks`
- `health_checks`

Pliki:

- `crates/domain/src/layout.rs`
- `data/layouts/*.de`

Efekt:

- layout zacznie być deklaratywnym artefaktem wdrożeniowym, a nie tylko skryptem

### 6. Rozbudować `LayoutTracker`

Cel:

- dziś tracker śledzi głównie zainstalowane rozszerzenia
- potrzebny jest pełniejszy stan wdrożenia

Nowy zakres:

- aktywny layout
- backend lub provider
- zainstalowane rozszerzenia przez aplikację
- włączone i wyłączone konflikty
- wynik ostatniej operacji
- snapshot before/after

Pliki:

- `crates/app/src/layout_tracker.rs`
- `crates/app/src/layout_service.rs`

Efekt:

- łatwiejszy rollback, diagnostyka i przyszłe migracje między layoutami

### 7. Wprowadzić jawne `health checks`

Cel:

- nie kończyć procesu na "krok wykonał się bez błędu"
- sprawdzać, czy system naprawdę osiągnął zamierzony stan

Typy health checks:

- aktywność rozszerzeń
- zgodność kluczy `gsettings`
- wynik poleceń kontrolnych
- poprawność stanu po shell reload

Pliki:

- `crates/domain/src/layout.rs`
- `crates/app/src/layout_service.rs`
- `crates/infra/src/executor.rs`
- `crates/infra/src/extensions.rs`

Efekt:

- wynik layoutu będzie oparty na stanie końcowym, a nie tylko na powodzeniu komend

### 8. Dodać pełny rollback

Cel:

- jeśli wdrożenie nie powiedzie się, system powinien umieć wrócić do poprzedniego stanu

Zakres:

- snapshot przed zmianą
- plan wycofania
- zapis raportu rollbacku
- integracja z historią

Pliki:

- `crates/app/src/snapshot_service.rs`
- `crates/app/src/history_service.rs`
- `crates/app/src/layout_service.rs`
- `crates/domain/src/history.rs`
- `crates/domain/src/snapshot.rs`

Efekt:

- wzrost bezpieczeństwa i wiarygodności narzędzia

## Rekomendowana kolejność wdrożenia

1. `ApplyResult` i poprawa obsługi błędów
2. `PreflightReport`
3. `ExecutionPlan`
4. rozszerzenie modelu `.de`
5. rozbudowa `LayoutTracker`
6. `health checks`
7. `rollback`

## Najważniejszy milestone

Najbliższym celem powinno być zbudowanie `safe apply pipeline`:

- `preflight`
- `plan`
- `execute`
- `verify`
- `rollback`
- `final report`

To jest najważniejsza zmiana, bo bez niej każde nowe layouty tylko powiększą obecny dług techniczny.
