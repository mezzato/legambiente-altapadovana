# Introduzione al software di monitoraggio delle polveri sottili

Data di aggiornamento: 2025.02.05

link utili:

- [pagina di visualizzazione dei dati](https://aerdigitalis.eu:3000/d/ee30edkvuqdc0c/pm2-5-pm10?var-timerange=10m&orgId=1&from=now-24h&to=now&timezone=browse&var-city=Vicenza&var-hidden_city2chip_id=$__all&var-sensor=$__all)
- [Sensor Community](https://sensor.community/it/)
- [Introduzione a Grafana (inglese)](https://grafana.com/docs/grafana/latest/getting-started/build-first-dashboard/)

## Come una centralina manda i dati

[un esempio di interfaccia della centralina](http://mezzsplace.ddns.net:4172/)

**DA RICORDARE**:

- ogni centralina puo' avere piu' sensori (polveri sottili - SDS011, temperatura - DHT22, ...)
- ogni sensore puo' fare piu' misure, per esempio le polveri sottili PM2.5 e PM10 insieme

Il formato dei dati e' di tipo testo, simile a:

```json
{
    "sensor": {
        "id": 62574,
        "pin": "1",
        "sensor_type": {
            "name": "SDS011",
            "id": 14,
            "manufacturer": "Nova Fitness"
        }
    },
    "sampling_rate": null,
    "id": 23720599478,
    "location": {
        "latitude": "45.63",
        "id": 77629,
        "exact_location": 0,
        "indoor": 0,
        "country": "IT",
        "altitude": "46.3",
        "longitude": "11.704"
    },
    "timestamp": "2025-02-02 17:28:13",
    "sensordatavalues": [
        {
            "value_type": "P1",
            "value": "33.33",
            "id": 54413549312
        },
        {
            "value_type": "P2",
            "value": "22.52",
            "id": 54413549348
        }
    ]
}
```

Le misure degli ultimi 5 min si possono scaricare per ogni **sensore**, per esempio: <https://data.sensor.community/airrohr/v1/sensor/62574/>

## I valori fondamentali

1. instante di misura (timestamp), per esempio: "2025-02-02 17:28:13"
2. il valore di una misura di un sensore, per esempio per il PM10 (P1 in codice):

   ```json
   {
     "value_type": "P1",
     "value": "33.33",
     "id": 54413549312
   }
   ```

## Dove vengono salvati i dati

In un database creato appositamente per salvare serie di misure temporali (time series), si chiama InfluxDX.

Per esempio: [influxdb per legambiente](https://static.125.41.201.195.clients.your-server.de:8086)

## Dove vengono visualizzati i dati

Grafana, che e' un tool opensource, attualmente qui: <https://aerdigitalis.eu:3000/>

username: legambiente

password: legambiente

### Dashboard

Grafana permette di creare delle dashboard, ossia delle pagine con visualizzazioni dei dati, per esempio quella ufficiale

[Valori PM10 e PM2.5 per centraline tracciate da Legambiente](https://aerdigitalis.eu:3000/d/ee30edkvuqdc0c/pm2-5-pm10?var-timerange=10m&orgId=1&from=now-24h&to=now&timezone=browse&var-city=Vicenza&var-hidden_city2chip_id=$__all&var-sensor=$__all)

## Che significato hanno i parametri della dashboard

- citta'
- sensore
- time range (finestra temporale)
- refresh

## Che valori vengono mostrati sui grafici

- media dei valori in un certo time range

Questo time range puo' essere:

1. media del valore sull'intero time range -> mappa
1. ogni 2 secondi -> time series (diagramma temporale)
1. media in fasce orarie (7-17) per giorno -> bar chart
1. media degli ultimi 10 minuti -> gauge


## Come aggiungere / togliere una centralina

Con gli opportuni diritti di accesso editare il file <https://github.com/mezzato/legambiente-altapadovana/blob/main/sensorcommunity/influxdb/sensors_by_city.csv> e salvare.


## Per curiosita' si possono vedere gli alert

<https://aerdigitalis.eu:3000/>
