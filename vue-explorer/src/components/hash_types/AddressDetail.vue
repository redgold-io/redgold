<template>

  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">

        <div>
          <div class="hash-container">
            <h3 class="detail-group">Address Details</h3>
          </div>
          <div class="grid-container">

            <div><strong>Address</strong></div>
            <div class="hash-container">
              {{ hashData.address }}
              <div><CopyClipboard :data="hashData.address" /></div>
            </div>

            <div><strong>Balance</strong></div>
            <div>{{ hashData.balance }} RDG</div>

            <div><strong>Total UTXOs</strong></div>
            <div>{{ hashData.total_utxos }}</div>

            <div><strong>Total Transactions</strong></div>
            <div>{{ hashData.total_count }}</div>

            <div><strong>Incoming Transactions</strong></div>
            <div>{{ hashData.incoming_count }}</div>

            <div><strong>Outgoing Transactions</strong></div>
            <div>{{ hashData.outgoing_count }}</div>

          </div>



          <div v-if="hashData.address_pool_info" >
            <h3 class="detail-group">AMM Swap Info</h3>
            <div class="grid-container">

              <div><strong>RDG Address</strong></div>
              <div><HashLink :shorten="false" :data="hashData.address_pool_info.addresses['Redgold']" /></div>
              <div><strong>RDG Address Balance</strong></div>
              <div>{{ parseFloat(hashData.address_pool_info.balances['Redgold'] || 0).toFixed(8)}} RDG</div>

              <div><strong>BTC Explorer Link</strong></div>
              <a :href="btcExplorerLink">{{btcExplorerLink}}</a>
              <div><strong>BTC Address</strong></div>
              <div><HashLink :shorten="false" :data="hashData.address_pool_info.addresses['Bitcoin']" /></div>
              <div><strong>BTC Balance</strong></div>
              <div>{{ parseFloat(hashData.address_pool_info.balances['Bitcoin'] || 0).toFixed(8) }} BTC</div>

              <div><strong>ETH Explorer Link</strong></div>
              <a :href="ethExplorerLink">{{ethExplorerLink}}</a>
              <div><strong>ETH Address</strong></div>
              <div><HashLink :shorten="false" :data="hashData.address_pool_info.addresses['Ethereum']" /></div>
              <div><strong>ETH Balance</strong></div>
              <div>{{ parseFloat(hashData.address_pool_info.balances['Ethereum'] || 0).toFixed(18) }} ETH</div>

              <div><strong>Public Key (Proto)</strong></div>
              <div><TextCopy :data="hashData.address_pool_info.public_key" /></div>

              <div><strong>Public Key (Compact)</strong></div>
              <div><TextCopy :data="publicKeyCompact" /></div>

              <div><strong>Price Ask USD/RDG BTC Quote</strong></div>
              <div><TextCopy :data="'$' + askPriceUsdRdg" /></div>
              <div><strong>Price Bid USD/RDG BTC Quote</strong></div>
              <div><TextCopy :data="'$' + bidPriceUsdRdg" /></div>
              <template v-for="balancePair in hashData.address_pool_info.overall_staking_balances" :key="balancePair[0]">
                <div><strong>{{balancePair[0]}} Staked</strong></div>
                <div>{{balancePair[1]}}</div>
              </template>
<!--              these are broken ?? displaying wrong balances. -->
<!--              <template v-for="balancePair in hashData.address_pool_info.amm_staking_balances" :key="balancePair[0]">-->
<!--                <div><strong>{{balancePair[0]}} AMM Staked</strong></div>-->
<!--                <div>{{balancePair[1]}}</div>-->
<!--              </template>-->
<!--              <template v-for="balancePair in hashData.address_pool_info.portfolio_staking_balances" :key="balancePair[0]">-->
<!--                <div><strong>{{balancePair[0]}} Port Staked</strong></div>-->
<!--                <div>{{balancePair[1]}}</div>-->
<!--              </template>-->

            </div>

            <h3 class="detail-group">Bid Ask AMM Curve RDG/BTC</h3>
            <div class="grid-container">
              <Bar :data="this.computeData('Bitcoin', false)" :options="exampleOptions" class="chart-container" />
              <Bar :data="this.computeData('Bitcoin', true)" :options="exampleOptions" class="chart-container" />
            </div>

            <h3 class="detail-group">Bid Ask AMM Curve RDG/ETH</h3>
            <div class="grid-container">
              <Bar :data="this.computeData('Ethereum', false)" :options="exampleOptions" class="chart-container" />
              <Bar :data="this.computeData('Ethereum', true)" :options="exampleOptions" class="chart-container" />
            </div>

            <h6 class="detail-group">Trade Calculator</h6>


            <!-- Cryptocurrency Selector -->
            <label v-if="calculatorTransactionType === 'BUY'">
              User Pair:
              <select v-model="userPair">
<!--                <option value="BTC">BTC</option>-->
<!--                <option value="ETH">ETH</option>-->
                <option value="USD">USD</option>
              </select>
            </label>

            <!-- BUY/SELL Radio Buttons -->

            <label>
              <input type="radio" v-model="calculatorTransactionType" value="BUY" /> BUY
            </label>
            <label>
              <input type="radio" v-model="calculatorTransactionType" value="SELL" /> SELL
            </label>

            <!-- Cryptocurrency Selector -->
            <label>
              Trade Pair:
              <select v-model="activeTradePair">
                <option value="Bitcoin" >BTC</option>
                <option value="Ethereum">ETH</option>
              </select>
            </label>


            <!-- Input Boxes -->
            <div v-if="calculatorTransactionType === 'BUY'">
              <label>
                {{ userPair }}:
                <input type="number" class="search-input" v-model="inputUser" />
              </label>
              <label>
                {{activeTradePair}}: {{ buyCalculatedAmount }}
              </label>
            </div>
            <div v-if="calculatorTransactionType === 'SELL'">
              <label>
                RDG:
                <input type="number" v-model="inputRDG" />
              </label>
            </div>

            <!-- Results Display -->
            <div class="horizontal-display">
              <span v-if="calculatorTransactionType === 'BUY'">You'll receive:</span>
              <TextCopy v-if="calculatorTransactionType === 'BUY'" :data="rdg_buy_amount.toFixed(8)" />
              <span v-if="calculatorTransactionType === 'BUY'">RDG</span>

              <span v-if="calculatorTransactionType === 'SELL'">You'll receive:</span>
              <TextCopy v-if="calculatorTransactionType === 'SELL'" :data="btc_sell_amount.toFixed(8)"/>
              <span v-if="calculatorTransactionType === 'SELL'">{{ activeTradePair }}</span>
            </div>


<!--            show events-->


          </div>


          <div class="flex-center">
            <h3 class="detail-group">Transactions</h3>
            <div class="radio-holder" style="display: inline-block; margin-left: 10px;">
              <label class="radio-option"><input type="radio" value="all" v-model="transactionType" /> All</label>
              <label class="radio-option"><input type="radio" value="incoming" v-model="transactionType" />  Incoming</label>
              <label class="radio-option"><input type="radio" value="outgoing" v-model="transactionType" /> Outgoing</label>
            </div>
          </div>
          <div><BriefTransaction :transactions="filteredTransactions" /></div>
          <nav>
            <ul class="pagination">
              <li class="page-item" :class="{ 'disabled': currentPage === 1 }">
                <a class="page-link" href="#" @click.prevent="goToPage(1)" :aria-disabled="currentPage === 1">First</a>
              </li>

              <li class="page-item" :class="{ 'disabled': currentPage === 1 }">
                <a class="page-link" href="#" @click.prevent="currentPage--" :aria-disabled="currentPage === 1">Previous</a>
              </li>

              <li class="page-item" v-for="page in visiblePages" :key="page" :class="{ 'active': page === currentPage }">
                <a class="page-link" href="#" @click.prevent="goToPage(page)">{{ page }}</a>
              </li>

              <li class="page-item" :class="{ 'disabled': currentPage === pageCount }">
                <a class="page-link" href="#" @click.prevent="currentPage++" :aria-disabled="currentPage === pageCount">Next</a>
              </li>

              <li class="page-item" :class="{ 'disabled': currentPage === pageCount }">
                <a class="page-link" href="#" @click.prevent="goToPage(pageCount)" :aria-disabled="currentPage === pageCount">Last</a>
              </li>

            </ul>
          </nav>


          <div v-if="hashData.address_pool_info" >
            <h3 class="detail-group">AMM Events</h3>
            <div v-if="hashData.address_pool_info.detailed_events">
              <div><DetailedEvent :events="hashData.address_pool_info.detailed_events" /></div>
            </div>
          </div>


        </div>
      </div>
    </div>
  </div>
</template>



<script>

// import HashLink from "@/components/HashLink.vue";
import CopyClipboard from "@/components/util/CopyClipboard.vue";
// import RenderTime from "@/components/RenderTime.vue";
import BriefTransaction from "@/components/BriefTransaction.vue";
import fetchHashInfo from "@/components/mixins/fetchHashInfo";
// import BidAskCurve from "@/components/BidAskCurve.vue";
// import LineWithLineChart from '@/components/LineWithLineChart.ts'
// import * as chartConfig from './chartConfig.js'
import {BarElement, CategoryScale, Chart as ChartJS, Legend, LinearScale, Title, Tooltip} from 'chart.js';
import {Bar} from 'vue-chartjs';
import TextCopy from "@/components/util/TextCopy.vue";
import HashLink from "@/components/util/HashLink.vue";
import DetailedEvent from "@/components/DetailedEvent.vue";

ChartJS.register(CategoryScale, LinearScale, BarElement, Title, Tooltip, Legend);
// ChartJS.defaults.global.defaultFontColor = '#FFFFFF';

export default {
  name: 'TransactionDetail',
  props: ['hashDataInitial'],
  components: {
    DetailedEvent,
    HashLink,
    TextCopy,
    BriefTransaction,
    // RenderTime,
    // HashLink,
    CopyClipboard,
    Bar
  },
  data: function() {
    return {
      inputUSD: null,
      inputPair: null,
      inputRDG: null,
      inputUser: null,
      buyCalculatedAmount: null,
      rdg_buy_amount: 0.0,
      btc_sell_amount: 0.0,
      updatingValue: false,
      lastEdited: null,  // Will hold either 'USD' or 'BTC'
      calculatorTransactionType: 'BUY', // Default value is set to 'BUY'
      // ... other data properties ...
      transactionType: 'all',
      currentPage: 1,
      perPage: 25,
      activeTradePair: 'Bitcoin',
      userPair: 'USD',
      hashData: this.hashDataInitial,
      exampleBidAskData: {
        labels: ['January', 'February', 'March', 'April', 'May', 'June', 'July', "", "", "", ""],
        datasets: [
          {
            label: 'Data One',
            backgroundColor: '#f87979',
            data: [40, 39, 10, 40, 39, 80, 40, 40, 39, 10, 40, 39, 80, 40]
          }
        ]
      },
      ask: {
        labels: ['January', 'February', 'March', 'April', 'May', 'June', 'July', "", "", "", ""],
        datasets: [
          {
            label: 'Data One',
            backgroundColor: '#f87979',
            data: [40, 39, 10, 40, 39, 80, 40, 40, 39, 10, 40, 39, 80, 40]
          }
        ]
      },
    }
  },
  watch: {

    inputUser(newUserValue) {
      let floatUserValue = parseFloat(newUserValue);
      if (this.calculatorTransactionType === 'BUY') {
        if (this.userPair === "USD") {
          if (this.activeTradePair === "Bitcoin") {
            this.buyCalculatedAmount = floatUserValue / this.usdBtcRate;
          }
          if (this.activeTradePair === "Ethereum") {
            this.buyCalculatedAmount = floatUserValue / this.usdEthRate;
          }
          this.inputPair = this.buyCalculatedAmount
        } else {
          if (this.userPair === "Bitcoin") {
            this.buyCalculatedAmount = floatUserValue * this.usdBtcRate;
          }
          if (this.userPair === "Ethereum") {
            this.buyCalculatedAmount = floatUserValue * this.usdEthRate;
          }
          this.inputPair = this.userPair
        }
      }
    },
    // inputUSD(newUSDValue) {
    //   // If the value is updated by the other watcher, do not recompute
    //   if (this.updatingValue || this.lastEdited === "BTC") return;
    //
    //   this.updatingValue = true;
    //   let floatUSDValue = parseFloat(newUSDValue);
    //   if (!isNaN(floatUSDValue)) {
    //     // console.log("New usd value " + floatUSDValue)
    //     if (this.activeTradePair === "Bitcoin") {
    //       this.inputPair = floatUSDValue / this.usdBtcRate;
    //     }
    //     if (this.activeTradePair === "Ethereum") {
    //       this.inputPair = floatUSDValue / this.usdEthRate;
    //     }
    //
    //     this.lastEdited = "USD";
    //     // console.log("New BTC value " + this.inputBTC)
    //
    //   }
    //   this.updatingValue = false;
    // },
    inputRDG(newRDGValue) {
      let floatRDGValue = parseFloat(newRDGValue);
      // console.log("New RDG value " + newRDGValue)
      if (this.hashData.address_pool_info != null && !(isNaN(floatRDGValue))) {

        let bids = this.hashData.address_pool_info.bids[this.activeTradePair];
        let total_rdg = floatRDGValue;
        let total_fulfilled = 0;
        // console.log("Total RDG: " + total_rdg)
        for (let i = 0; i < bids.length; i++) {
          // console.log("i: " + i)
          let bid = bids[i];
          let p_i = bid.price // RDG / BTC
          let p = 1 / p_i // BTC / RDG
          let v = bid.volume / 1e8 // amount BTC available for purchase via RDG
          let requested_vol = total_rdg * p;
          // console.log("Requested vol: " + requested_vol)
          // console.log("bid: " + bid)
          // console.log("inverse_p: " + p)
          // console.log("v: " + v)
          if (requested_vol > v) {
            total_rdg -= v / p;
            total_fulfilled += v
          } else {
            total_rdg = 0;
            total_fulfilled += requested_vol
            break
          }
        }
        this.btc_sell_amount = total_fulfilled;
      }
    },
      inputPair(newBTCValue) {
        let floatBtcValue = parseFloat(newBTCValue);

        if (this.hashData.address_pool_info != null && !(isNaN(floatBtcValue))) {
          let asks = this.hashData.address_pool_info.asks[this.activeTradePair];
          let total_btc = floatBtcValue;
          let total_fulfilled = 0;
          for (let i = 0; i < asks.length; i++) {
            let ask = asks[i];
            let p = ask.price // RDG / BTC now
            let v = ask.volume / 1e8 // amount RDG available for sale via ask
            let requested_vol = total_btc * p // BTC * (RDG/BTC) = vol RDG unit;
            let thisBtc = v / p; // RDG / RDG / BTC = BTC
            console.log(`ask ${ask} p ${p} v ${v} requested_vol ${requested_vol}
            thisBtc ${thisBtc} total_btc ${total_btc} total_fulfilled ${total_fulfilled} float_btc_value ${floatBtcValue}`)
            if (requested_vol > v) {
              total_btc -= thisBtc;
              total_fulfilled += v
            } else {
              total_btc = 0;
              total_fulfilled += requested_vol
              break
            }
          }
          this.rdg_buy_amount = total_fulfilled;
        }

        // If the value is updated by the other watcher, do not recompute
        if (this.updatingValue) return;

        this.updatingValue = true;
        if (!isNaN(floatBtcValue)) {
          // console.log("New usd value " + floatBtcValue)
          this.inputUSD = floatBtcValue * this.usdBtcRate;
        }
        this.updatingValue = false;
      }
  },
  mixins: [fetchHashInfo],
  computed: {

    publicKeyCompact() {
      let excludePrefixes = ['0a220a20', '0a230a2103', '0a230a2102']
      let dat = this.hashData.address_pool_info.public_key;

        for (let pfx of excludePrefixes) {
          if (dat.startsWith(pfx)) {
            return dat.substring(pfx.length);
          }
        }
      return dat
    },
    btcExplorerLink() {

      var net = "testnet/";
      let btcAddress = this.hashData.address_pool_info.addresses['Bitcoin'];

      if (!btcAddress.startsWith("tb")) {
        net = "";
      }

      return "https://blockstream.info/" + net + "address/" + btcAddress;
    },
    ethExplorerLink() {
      let btcAddress = this.hashData.address_pool_info.addresses['Bitcoin'];
      let ethAddress = this.hashData.address_pool_info.addresses['Ethereum'];
      var retUrl = "https://sepolia.etherscan.io/address/" + ethAddress;
      if (!btcAddress.startsWith("tb")) {
        retUrl = "https://etherscan.io/address/" + ethAddress;
      }
      return retUrl;
    },

    bidPriceUsdRdg() {
      if (this.hashData.address_pool_info != null) {
        let centerPrice = this.hashData.address_pool_info.central_prices['Bitcoin'];
        if (centerPrice != null) {
          return centerPrice.min_bid_estimated.toFixed(2);
        }
      }
      return 0;
    },
    //
    // centerPriceRdgBtc() {
    //   if (this.hashData.address_pool_info != null) {
    //     let centerPrice = this.hashData.address_pool_info.bid_ask.center_price;
    //     let usdPrice = (1/centerPrice) * this.usdBtcRate;
    //     return usdPrice.toFixed(2);
    //   }
    //   return 0;
    // },

    askPriceUsdRdg() {
      if (this.hashData.address_pool_info != null) {
        let centerPrice = this.hashData.address_pool_info.central_prices['Bitcoin'];
        if (centerPrice != null) {
          return centerPrice.min_ask_estimated.toFixed(2);
        }
      }
      return 0;
    },
    // spreadUsd() {
    //   if (this.hashData.address_pool_info != null) {
    //     let ba = this.hashData.address_pool_info.bid_ask;
    //     if (ba.asks.length > 0 && ba.bids.length > 0) {
    //       let ask_first = ba.asks[0].price; // BTC / RDG
    //       let bid_first = ba.bids[0].price; // RDG / BTC
    //       let adjusted_bid = 1/bid_first; // BTC / RDG
    //       let usd_ask = ask_first * this.usdBtcRate
    //       let usd_bid = adjusted_bid * this.usdBtcRate
    //       return (usd_ask - usd_bid).toFixed(2);
    //     }
    //   }
    //   return "na";
    // },
    usdBtcRate() {
      return this.$store.state.btcExchangeRate;
    },
    usdEthRate() {
      return this.$store.state.ethExchangeRate;
    },
    exampleOptions(){
      return {
      responsive: false,
      maintainAspectRatio: true,
      // defaultFontColor: '#FFFFFF',
      // scales: {
      //   xAxes: [{
      //     ticks: {
      //       fontColor: '#FFFFFF'
      //     }
      //   }],
      //   yAxes: [{
      //     ticks: {
      //       fontColor: '#FFFFFF'
      //     }
      //   }]
      // },
      // legend: {
      //   labels: {
      //     fontColor: '#FFFFFF'
      //   }
      // },
      tooltips: {
        titleFontColor: '#FFFFFF',
        bodyFontColor: '#FFFFFF',
        footerFontColor: '#FFFFFF',
        callbacks: {
          label: ((tooltipItems, data) => {
            console.log(this)
            return tooltipItems.yLabel + '£ yo ' + data;
          }),
          title: ((toolTipItems, data) => {
            return "WTF " + toolTipItems + data;
          })
        }
        // callbacks: {
        //   title: function(tooltipItems, data) {
        //     // Return the label for the current item
        //     return "title: " + data.labels[tooltipItems[0].index];
        //   },
        //   label: function(tooltipItem, data) {
        //     // Return the value for the current item
        //     return "label: " + data.datasets[tooltipItem.datasetIndex].data[tooltipItem.index];
        //   }
        // }
      },
    }},
    computedBidData() {
      return this.computeData("Bitcoin", false)
      // let labels = [];
      // let data = [];
      // let api = this.hashData.address_pool_info;
      // if (api != null) {
      //   let ba = api.bid_ask;
      //   // console.log("Bid ask: " + ba);
      //   if (ba != null) {
      //     let bids = ba.bids;
      //     if (bids != null) {
      //       for (let i = 0; i < bids.length; i++) {
      //         let bid = bids[i];
      //         // console.log("Bid " + bid);
      //         if (bid.price != null) {
      //           // Price is originally in RDG / BTC -- i.e. 400 RDG / 1 BTC
      //           // We want to convert it to USD / RDG
      //           let rdg_btc = bid.price; // RDG / BTC
      //           let btc_rdg = (1 / rdg_btc); // BTC / RDG
      //           let usd_btc = this.usdBtcRate; // USD / BTC
      //           let price = btc_rdg * usd_btc; // USD / RDG
      //           labels.push(price);
      //         }
      //         if (bid.volume != null) {
      //           data.push(bid.volume);
      //         }
      //       }
      //     }
      //   }
      // }
      // while (labels.length < 25) {
      //   labels.push(0);
      // }
      // while (data.length < 25) {
      //   data.push(0);
      // }
      //
      // let slice_len = 25;
      // let resultLabels = labels.map(value => {
      //   return value.toFixed(2);
      // }).slice(0, slice_len).reverse();
      // let resultData = data.map(value => {
      //   return value.toFixed(2);
      // }).slice(0, slice_len).reverse();
      // // console.log("Result labels: " + resultLabels);
      // // console.log("Result data: " + resultData);
      // return {
      //   labels: resultLabels,
      //   datasets: [
      //     {
      //       label: 'BTC Bid USD/Volume(Sats)',
      //       backgroundColor: '#79f87f',
      //       data: resultData
      //     }
      //   ]
      // }
    },

    computedAskData() {
      return this.computeData("Bitcoin", true)
    },
    filteredTransactions() {
      if (this.transactionType === 'incoming') {
        return this.hashData.incoming_transactions;
      } else if (this.transactionType === 'outgoing') {
        return this.hashData.outgoing_transactions;
      }
      return this.hashData.recent_transactions;
    },
    numTransactions() {
      if (this.transactionType === 'incoming') {
        return this.hashData.incoming_count;
      } else if (this.transactionType === 'outgoing') {
        return this.hashData.outgoing_count;
      }
      return this.hashData.total_count;
    },
    pageCount() {
      return Math.ceil(this.numTransactions / this.perPage);
    },
    visiblePages() {
      let startPage = Math.max(1, this.currentPage - 5);
      let endPage = Math.min(this.pageCount, this.currentPage + 5);

      let pages = [];
      for (let i = startPage; i <= endPage; i++) {
        pages.push(i);
      }
      return pages;
    },
  },
  methods: {
    computeData(pair, isAsk) {
      let labels = [];
      let data = [];
      let api = this.hashData.address_pool_info;
      if (api != null) {
        let ba = isAsk ? api.asks_usd : api.bids_usd;
        if (ba != null) {
          let asks = ba[pair];
          if (asks != null) {
            for (let i = 0; i < asks.length; i++) {
              let ask = asks[i];
              // console.log("Bid " + ask);
              if (ask.price != null) {
                let usdPrice = ask.price; // USD / RDG
                labels.push(usdPrice);
              }
              if (ask.volume != null) {
                let vol = isAsk ? (ask.volume / 1e8) : ask.volume;
                data.push(vol);
              }
            }
          }
        }
      }
      let slice_len = 25;

      if (!isAsk) {
        while (labels.length < slice_len) {
          labels.push(0);
        }
        while (data.length < slice_len) {
          data.push(0);
        }
      }

      let resultLabels = labels.map(value => {
        return value.toFixed(2);
      }).slice(0, slice_len);
      let resultData = data.map(value => {
        return value.toFixed(2);
      }).slice(0, slice_len);
      if (!isAsk) {
        resultLabels = resultLabels.reverse();
        resultData = resultData.reverse();
        console.log("Result asks labels: " + resultLabels);
        console.log("Result asks data: " + resultData);
      }

      return {
        labels: resultLabels,
        datasets: [
          {
            label: isAsk ? 'Ask USD/Volume(RDG)' : 'Bid USD/Volume(Sats)',
            backgroundColor: isAsk? '#f87979' : '#79f87f',
            data: resultData
          }
        ]
      }
    },
    // preprocessData(bids, asks) {
    //   // Sort bids and asks
    //   bids = bids.sort((a, b) => b[0] - a[0]);
    //   asks = asks.sort((a, b) => a[0] - b[0]);
    //
    //   let bidPrices = bids.map(item => item[0]);
    //   let bidQuantities = bids.map(item => item[1].cumulative || item[1]);
    //
    //   let askPrices = asks.map(item => item[0]);
    //   let askQuantities = asks.map(item => item[1].cumulative || item[1]);
    //
    //   return {
    //     labels: [...bidPrices, ...askPrices],
    //     datasets: [
    //       {
    //         label: 'Bids',
    //         data: bidQuantities,
    //         borderColor: 'green',
    //         fill: false
    //       },
    //       {
    //         label: 'Asks',
    //         data: askQuantities,
    //         borderColor: 'red',
    //         fill: false
    //       }
    //     ]
    //   }
    // },
    async goToPage(page) {
      if (page !== this.currentPage) {
        this.currentPage = page;

        // Calculate offset and limit for fetching data
        let offset = (this.currentPage - 1) * this.perPage;
        let limit = this.perPage;

        // Fetch data and update hashData
        await this.fetchData(offset, limit)
      }
    }
  }

}
</script>

<style scoped>
.grid-container {
  display: grid;
  grid-template-columns: 1fr 6fr; /* Adjust as needed */
  gap: 10px; /* Adjust as needed */
  padding-top: 5px;
  padding-bottom: 5px;
}

.hash-container {
  display: flex;
  align-items: center;
}

.flex-center {
  display: flex;
  align-items: center;
}


.detail-group {
  padding-top: 15px;
  padding-bottom: 15px;
  padding-left: 10px;
  padding-right: 20px;
  background-color: #191a19 !important;
}
.signature {
  word-break: break-word;
  overflow-wrap: break-word;
}

.radio-option {
  margin-right: 20px;
}

.radio-option input[type="radio"] {
  //display: none;
}

.radio-option span {
  padding: 10px;
  border: 1px solid #ccc;
  display: inline-block;
  margin-right: 5px;
  cursor: pointer;
}

.radio-option input[type="radio"]:checked + span {
  //background-color: #ddd;
}

.radio-holder {
  padding-left: 10px;
  background-color: #191a19 !important;
}

.pagination {
  background-color: #000000; /* slightly lighter grey for the active page */
}

.page-link {
  color: #fff; /* white text */
  background-color: #000000; /* slightly lighter grey for the active page */
}

.page-item.active .page-link {
  background-color: #000000; /* slightly lighter grey for the active page */
  border-color: #666;
}

.page-item.disabled .page-link {
  color: #999; /* lighter grey text for disabled buttons */
}

.chart-container {
  //position: relative; /* Important for responsive sizing */
  height: 600px;
  width: 600px;
  color: #FFFFFF;
}

label {
  margin-right: 20px; /* Adjust the value as per your requirement */
}


.search-bar {
  background-color: #000;
}

.search-input,
.search-input:focus {
  box-sizing: border-box;
  min-width: 200px;
  max-width: 200px;
  background-color: #191a19;
  color: #fff;
}
.search-input::placeholder {
  color: #ccc;
}

/* This will space out each <label> element */
label {
  //display: block; /* Makes labels appear on new lines */
  margin-bottom: 10px; /* Adjust this value as per your preference */
}

/* This gives space below your headers and results */
h6, .detail-group, div {
  margin-bottom: 10px; /* Adjust this value as per your preference */
}

.horizontal-display {
  display: flex;
  align-items: normal; /* Vertically aligns the items in the center */
  gap: 10px; /* Space between the items, adjust as needed */
}

</style>