# Screen-PDS

Questa repository contiene il codice sorgente del progetto del corso di Programmazione di sistema 2023, sviluppato da Alessandro Fedriga (s320136) e Salvatore Francesco Rossetta (s317876). L'applicazione è stata sviluppata per `Windows`, `Linux (X11)` e `MacOS`, anche se è stata testata principalmente su `Windows 11` e `Linux (Ubuntu 23)`.

## Requisiti 
La versione minima di Rust supportata è `1.71.1`. 
La versione minima di GTK4 supportata è `4.10`.

Sono stati utilizzati inoltre crate open-source presenti nel `Cargo.toml`, dove sono già specificate le versioni richieste per l'applicazione.   

## Dipendenze 

### Linux
Per eseguire correttamente su Linux, è necessario installare le librerie `libxcb`, `libxrandr` e `dbus` tramite il seguente comando: 

```
apt-get install libxcb1 libxrandr2 libdbus-1-3
```


### Windows 
Per eseguire correttamente su Windows, è necessario installare gtk4 seguendo [questa guida](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_windows.html).

### MacOS 
Per eseguire correttamente su MacOS, è necessario installare innanzitutto rustup:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Poi, se non presente, homebrew: 

```
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Se non già presente, pkg-config:

```
brew install pkg-config
```

Infine gtk4: 

```
brew install gtk4
```


## Utilizzo
L'applicazione permette di creare screenshots con possibili delay, ritagliare successivamente l'immagine e salvarla in diversi formati. 
All'avvio si presenta una schermata con dei bottoni che permettono di creare e salvare un'immagine, selezionare un delay e il formato desiderato.
Inoltre, sono presenti delle hotkeys per creare e salvare un'immagine, che possono essere personalizzate dall'utente nelle impostazioni ed è possibile 
cambiare il percorso di default in cui salvare l'immagine.

## Struttura
Per compilare ed eseguire il codice, la struttura della cartella deve essere come segue:
```
screen-pds
|_ src
|  |_ main.rs
|_ settings.json
|_ cargo.toml

```