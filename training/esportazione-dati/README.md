# Come esportare dati delle polveri sottili

## Esportare da Sensor Community come file CSV ed importare in Google Sheets

### Esportare da Sensor Community come file CSV

Individuare l'ID del sensore dalla mappa di Sensor Community (88091 nell'immagine):

![check action execution](./media/sensor_id.png)

Scaricare i dati giornalieri dal link: 

```
https://archive.sensor.community/[data in formato dd-mm-yyyy]/[data in formato dd-mm-yyyy]_sds011_sensor_[ID].csv
```

per esempio per il sensore 88091, il giorno 22-02-2026:

```
https://archive.sensor.community/2026-02-22/2026-02-22_sds011_sensor_88091.csv
```

I dati sono in formato CSV, per esempio:

```csv
sensor_id;sensor_type;location;lat;lon;timestamp;P1;durP1;ratioP1;P2;durP2;ratioP2
88091;SDS011;85606;45.528;11.560;2026-02-22T00:00:44;47.40;;;23.70;;
88091;SDS011;85606;45.528;11.560;2026-02-22T00:03:10;48.17;;;24.80;;
88091;SDS011;85606;45.528;11.560;2026-02-22T00:05:36;52.93;;;23.90;;
88091;SDS011;85606;45.528;11.560;2026-02-22T00:08:03;50.35;;;25.63;;
88091;SDS011;85606;45.528;11.560;2026-02-22T00:10:29;50.20;;;25.65;;
...
```

I dati rilevanti sono le colonne: timestamp (data), P1 (PM10), P2 (PM2.5).


### Importare i dati in Google Sheets da file CSV

I file CSV usano un formato numerico decimale con ".", come negli US. Quindi e' consigliabile cambiare (temporaneamente) la lingua prima di importare i dati.

![change language](./media/google-sheet-language.png)

Quindi importare i dati:

![import popup](./media/google-sheet-import-menu.png)

![import settings](./media/google-sheet-import-settings.png)

Per creare un grafico per esempio selezionare l'area dati voluta con i titoli di colonna ed inserire un grafico:

![import settings](./media/google-sheet-chart.png)

## Esportare dal sito di Grafana di Legambiente

Questo metodo permette una ricerca temporale piu' agevole, perche' i dati di Sensor Community possono solo essere esportati giorno per giorno.