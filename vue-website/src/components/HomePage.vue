<template>
  <div class="container-fluid">
    <div class="row no-gutters align-items-center desktop-buffer">
      <!-- Intro text container -->
      <div class="col-12 col-md-6 order-1 order-md-1 px-md-3">
        <div class="intro-text">
          <h2 style="margin-top: 10px">Decentralized Portfolio Contracts & Data Lake</h2>
          <p class="sub-intro-text">Redgold is a platform for building data driven crypto applications and smart contracts, with
            a focus on ETFs, portfolio models & portfolio contracts.
<!--          </p>-->
<!--          <p class="sub-intro-text">-->
            Native multiparty encryption support, customized financial ratings, and real-time pricing data. Construct your own
            user-defined native crypto ETFs using data stores, analytics queries, and SQL backed smart contracts.
          </p>
          <div class="d-flex intro-buttons">
<!--            justify-content-center -->
            <a href="https://dev.docs.redgold.io/introduction/getting-started" class="btn btn-primary intro-button">Start</a>
            <a href="https://dev.docs.redgold.io/whitepaper/introduction" class="btn btn-primary intro-button" >Paper</a>
            <a href="https://dev.docs.redgold.io/guides/amm-swaps-exchange" class="btn btn-primary intro-button" >Buy</a>
          </div>
        </div>
      </div>
      <!-- Image container -->
      <div class="col-12 col-md-6 order-2 order-md-2 px-md-3">
        <img class="splash-image" :src="require('@/assets/dalle-home-splash.webp')" alt="Splash Image">
      </div>
    </div>

    <div class="row second-element">
      <div class="col-12 text-center">
        <div class="powered-by-container">
          <p class="powered-by-text">POWERED BY</p>
          <div class="logos-container d-flex justify-content-center align-items-center flex-wrap">
            <!-- Repeat for each logo -->
            <div class="logo">
              <img :src="require('@/assets/powered-by/rust.webp')" alt="Rust Logo" class="powered-by-logo" >
            </div>
            <div class="logo">
              <img :src="require('@/assets/powered-by/tokio.png')" alt="Tokio Logo" class="powered-by-logo" >
            </div>
            <div class="logo">
              <img :src="require('@/assets/powered-by/sqlite.png')" alt="SQLite Logo" class="powered-by-logo" style="height: 50px; width: auto;">
            </div>
            <div class="logo">
              <img :src="require('@/assets/powered-by/protobuf.webp')" alt="Protobuf Logo" class="powered-by-logo" >
            </div>

            <div class="logo">
              <img :src="require('@/assets/powered-by/webassembly.webp')" alt="WebAssembly Logo" class="powered-by-logo" >
            </div>

<!--            <div class="logo">-->
<!--              <img :src="require('@/assets/powered-by/sql.webp')" alt="SQL Logo" class="powered-by-logo" style="height: 300%; width: auto;">-->
<!--            </div>-->

            <div class="logo">
              <img :src="require('@/assets/powered-by/parquet.webp')" alt="Parquet Logo" class="powered-by-logo" style="height: 40px; width: auto;">
            </div>

            <div class="logo">
              <img :src="require('@/assets/powered-by/arrow.webp')" alt="Arrow Logo" class="powered-by-logo" >
            </div>

<!--            <div class="logo">-->
<!--              <img :src="require('@/assets/powered-by/ebpf.webp')" alt="eBPF Logo" class="powered-by-logo" >-->
<!--            </div>-->

            <div class="logo">
              <img :src="require('@/assets/powered-by/docker.png')" alt="docker Logo" class="powered-by-logo" >
            </div>

            <!-- ... Additional logos ... -->
          </div>
        </div>
      </div>
    </div>
    <div class="row no-gutters align-items-center desktop-buffer">
      <div class="col-12 col-md-6 order-2 order-md-1 px-md-3">
        <div class="intro-text">
<!--          <h2>Peer-to-Peer Data Lake & Compute Engine</h2>-->
          <p class="sub-intro-text-section2">
            Applications are meant to be stateless and isolated from data stores.
            Typical contracts persist internal variable state through serialization between successive invocations,
            a strategy which would be considered an anti-pattern for a conventional application.
          </p>
          <p class="sub-intro-text-section2">
          Instead, SQL & database like queries and data transformations should be supported as a first class citizen,
            with support for state isolation & application data model development.
            Blockchains are supposed to be a world-database, and yet very little of that functionality is exposed to the end-user.
          </p>
          <div class="d-flex justify-content-center intro-buttons">
            <a href="https://dev.docs.redgold.io/introduction/about" class="btn btn-primary intro-button">Read more in the About section</a>
<!--            <a href="https://dev.docs.redgold.io/whitepaper/introduction" class="btn btn-primary intro-button" >Whitepaper</a>-->
          </div>
        </div>
      </div>
      <!-- Image container -->
      <div class="col-12 col-md-6 order-1 order-md-2 px-md-3">
        <img class="splash-image-smaller" :src="require('@/assets/main-splash-images/world-database.webp')" alt="Splash Image">
      </div>
  </div>

    <div class="row no-gutters align-items-center desktop-buffer">

      <!-- Image container -->
      <div class="col-12 col-md-6 order-2 order-md-1 px-md-3">
        <img class="splash-image-smaller" :src="require('@/assets/main-splash-images/fair-fees.webp')" alt="Splash Image">
      </div>
      <div class="col-12 col-md-6 order-1 order-md-2 px-md-3">
        <div class="intro-text">
                    <h2>MARKETPLACE FOR NOTARIZATION</h2>
          <p class="sub-intro-text-section2">
            Blockchains should not be companies, but instead reflect an openly audited market for validation. Nodes
            should compete to validate transactions, and fees should be set by market forces and ratings amongst each
            other. Rather than Foundations or companies or closed DAOs, individual peer decisions should shape
            the behavior.
          </p>
          <p class="sub-intro-text-section2">
            Fees should not reflect ownership in a decentralized business, but rather user preferences and behavior.
            Large ownership interests shouldn't unfairly determine policy to the detriment of other users.
          </p>
          <div class="d-flex justify-content-center intro-buttons">
<!--            <a href="https://dev.docs.redgold.io/introduction/about" class="btn btn-primary intro-button">Read more in the About section</a>-->
            <!--            <a href="https://dev.docs.redgold.io/whitepaper/introduction" class="btn btn-primary intro-button" >Whitepaper</a>-->
          </div>
        </div>
      </div>
    </div>

    <div class="row no-gutters align-items-center desktop-buffer second-element">

      <!-- Image container -->
      <div class="col-12 col-md-6 order-2 order-md-1 px-md-3">
        <img class="splash-image-smaller" :src="require('@/assets/main-splash-images/relative-models.webp')" alt="Splash Image">
      </div>
      <div class="col-12 col-md-6 order-1 order-md-2 px-md-3">
        <div class="intro-text">
                    <h2>RELATIVE MODELS</h2>
          <p class="sub-intro-text-section2">
            Centralized data structures containing the entire state of the world and all transactions can be avoided
            entirely. There are very few reasons to have a total ordering of completely independent transactions,
            and few situations in real life where complete agreement is realistic.

          </p>
          <p class="sub-intro-text-section2">
            Conflicts alone are what should be focused on, and can be propagated and resolved independently,
            without the enforcement of a global state. Total hash states necessitate absolute agreement, leading to
            numerous problems. Local relative ordering leads to both horizontal scalability, and the ability for
            nodes to cooperate despite their disagreements.
          </p>
          <div class="d-flex justify-content-center intro-buttons">
<!--            <a href="https://dev.docs.redgold.io/introduction/about" class="btn btn-primary intro-button">Read more in the About section</a>-->
            <!--            <a href="https://dev.docs.redgold.io/whitepaper/introduction" class="btn btn-primary intro-button" >Whitepaper</a>-->
          </div>
        </div>
      </div>
    </div>


    <div class="row no-gutters align-items-center desktop-buffer second-element">


      <div class="col-12 col-md-6 order-2 order-md-2 px-md-3">
      <!-- Image container -->
        <img class="splash-image-smaller" :src="require('@/assets/main-splash-images/portfolio-target-models.webp')" alt="Splash Image">
      </div>
     <div class="col-12 col-md-6 order-1 order-md-1 px-md-3">

        <div class="intro-text">
          <h2>PORTFOLIO TARGET MODELS</h2>
          <p class="sub-intro-text-section2">
            Finance still remains the main use case of crypto. Support for ETFs, customizable trading models,
            strategies and derivative products is lacking.
            Bridge risk models, custodial redemption, and oracle pricing are relative operations, and need a new model for representation & safety.

          </p>
          <p class="sub-intro-text-section2">
            Developments in t-ECDSA and other multiparty cryptographic operations can be used to create many unique
            decentralized groups of machines holding native assets and executing complex financial operations through
            smart contracts with access to an open data lake. Define your idealized portfolio, rate peers, and
            automatically rebalance based upon your own data-driven criteria.
          </p>
          <div class="d-flex justify-content-center intro-buttons">
            <!--            <a href="https://dev.docs.redgold.io/introduction/about" class="btn btn-primary intro-button">Read more in the About section</a>-->
            <!--            <a href="https://dev.docs.redgold.io/whitepaper/introduction" class="btn btn-primary intro-button" >Whitepaper</a>-->
          </div>
        </div>
      </div>
    </div>


    <div class="row no-gutters align-items-center desktop-buffer">

      <!-- Image container -->
      <div class="col-12 col-md-6 order-2 order-md-1 px-md-3">
        <img class="splash-image-smaller" :src="require('@/assets/main-splash-images/data-pipeline.webp')" alt="Splash Image">
      </div>
      <div class="col-12 col-md-6 order-1 order-md-2 px-md-3">
        <div class="intro-text">
          <h2>DATA PIPELINES</h2>
          <p class="sub-intro-text-section2">
            Using the eUTXO model, data is naturally expressed as a series of transforms. Oracle sourced
            pricing information or other data is treated equivalently to modern database operations. Define
            your own custom data sources with WASM crawlers, implement parsers, and customize a pipeline around
            your own unique use case.
          </p>
          <p class="sub-intro-text-section2">
            Transform APIs allow connecting SQL or other readers to UDFs to more complex mapping transforms in a
            seamless unified experience. Generic executors allow multiple languages & re-use of existing code
          </p>
          <div class="d-flex justify-content-center intro-buttons">
            <!--            <a href="https://dev.docs.redgold.io/introduction/about" class="btn btn-primary intro-button">Read more in the About section</a>-->
            <!--            <a href="https://dev.docs.redgold.io/whitepaper/introduction" class="btn btn-primary intro-button" >Whitepaper</a>-->
          </div>
        </div>
      </div>
    </div>

    <div class="row no-gutters align-items-center desktop-buffer">

      <!-- Image container -->
      <div class="col-12 col-md-6 order-2 order-md-1 px-md-3">
        <img class="splash-image-smaller" :src="require('@/assets/main-splash-images/why-redgold.webp')" alt="Splash Image">
      </div>
      <div class="col-12 col-md-6 order-1 order-md-2 px-md-3">
        <div class="intro-text">
          <h2>WHY REDGOLD</h2>
          <p class="sub-intro-text-section2">
            A different approach, using relative peer to peer ratings & scores
          </p>
          <p class="sub-intro-text-section2">
            Social proof is the underlying phenomena used to prevent exceptions in PoW and PoS systems, and yet this process is obfuscated, manual, and error prone.
            A proper model should account for relative peer decisions in order to avoid centralization and conflicts
          </p>
          <div class="d-flex justify-content-center intro-buttons">
            <!--            <a href="https://dev.docs.redgold.io/introduction/about" class="btn btn-primary intro-button">Read more in the About section</a>-->
            <!--            <a href="https://dev.docs.redgold.io/whitepaper/introduction" class="btn btn-primary intro-button" >Whitepaper</a>-->
          </div>
        </div>
      </div>
    </div>


    <div class="row no-gutters align-items-center desktop-buffer second-element">


      <div class="col-12 col-md-6 order-2 order-md-2 px-md-3">
        <!-- Image container -->
        <img class="splash-image-smaller" :src="require('@/assets/main-splash-images/secure-scaling.webp')" alt="Splash Image">
      </div>
      <div class="col-12 col-md-6 order-1 order-md-1 px-md-3">

        <div class="intro-text">
          <h2>SECURE SCALING</h2>
          <p class="sub-intro-text-section2">
            Hash Distance: Local decisions based on node id determine data persistence for fuzzy sharding and redundancy
          </p>
          <p class="sub-intro-text-section2">
            Dynamic Fees: Peers flexibly attempt to prioritize any transaction they have an incentive to verify in an open market
          </p>
          <p class="sub-intro-text-section2">
            Merkle Proof: End result is a collection of Merkle proofs proving acceptance of a given transaction from peers
          </p>
          <p class="sub-intro-text-section2">
            Small Nodes: Workers don't need to be supercomputers to participate and produce proofs
          </p>
          <a href="https://arxiv.org/pdf/2108.05236.pdf">ACCEPT based horizontal scaling</a>

          <div class="d-flex justify-content-center intro-buttons">
            <!--            <a href="https://dev.docs.redgold.io/introduction/about" class="btn btn-primary intro-button">Read more in the About section</a>-->
            <!--            <a href="https://dev.docs.redgold.io/whitepaper/introduction" class="btn btn-primary intro-button" >Whitepaper</a>-->
          </div>
        </div>
      </div>
    </div>

<!--    Footer -->
    <div class="footer-container">
      <div class="row no-gutters align-items-center desktop-buffer second-element footer">
        <div class="col-12 col-md-4 order-1 order-md-2 px-md-3"><p>Copyright Redgold Labs 2022</p></div>
        <div class="col-12 col-md-4 order-2 order-md-2 px-md-3"><a href="info@redgold.io">info@redgold.io</a></div>
        <div class="col-12 col-md-4 order-3 order-md-3 px-md-3"><a href="https://github.com/redgold-io/redgold">GitHub</a></div>
      </div>
    </div>

    <!--    <div class="row no-gutters align-items-center desktop-buffer second-element">-->
<!--      &lt;!&ndash; Intro text container &ndash;&gt;-->
<!--      <div class="col-12 col-md-7 order-2 order-md-1 px-md-3">-->
<!--        <div class="intro-text">-->
<!--          <h2>Peer-to-Peer Data Lake & Compute Engine</h2>-->
<!--          <p class="sub-intro-text">-->
<!--            Redgold is a decentralized platform for building data-oriented crypto applications and smart contracts, with-->
<!--            a focus on ETFs & portfolio models.-->
<!--          </p>-->
<!--          <p>-->
<!--            With multiparty encryption support and customized financial ratings, construct your own-->
<!--            user-defined native crypto ETFs, data stores, analytics, and SQL backed smart contracts.-->
<!--          </p>-->
<!--          <div class="d-flex justify-content-center intro-buttons">-->
<!--            <a href="https://dev.docs.redgold.io/introduction/getting-started" class="btn btn-primary intro-button">Get Started</a>-->
<!--            <a href="https://dev.docs.redgold.io/whitepaper/introduction" class="btn btn-primary intro-button" >Whitepaper</a>-->
<!--          </div>-->
<!--        </div>-->
<!--      </div>-->
<!--      &lt;!&ndash; Image container &ndash;&gt;-->
<!--      <div class="col-12 col-md-5 order-1 order-md-2 px-md-3">-->
<!--        <img class="splash-image" :src="require('@/assets/dalle-home-splash.webp')" alt="Splash Image">-->
<!--      </div>-->
<!--    </div>-->
  </div>
</template>
<style>

.footer {
  background-color: #121111;
  color: #fff;
  padding: 60px;
  text-align: left;
}

.powered-by-container {
  margin-top: 20px; /* Adjust as needed */
  margin-bottom: 20px; /* Adjust as needed */
}

.powered-by-text {
  font-weight: bold;
  text-transform: uppercase;
  margin-bottom: 15px; /* Space between text and logos */
}

.logos-container .logo {
  padding: 10px; /* Creates space around each logo */
}

.powered-by-logo {
  max-height: 85px; /* Adjust based on your logo sizes */
  margin: 0 15px; /* Horizontal space between logos */
  vertical-align: middle; /* Aligns logos vertically if they are different heights */
}

/* Responsive adjustments if needed */
@media (max-width: 768px) {
  .powered-by-logo {
    margin: 10px 5px; /* Less horizontal space on smaller screens */
  }
}

@media (min-width: 768px) { /* Adjusts to Bootstrap's MD breakpoint */
  .desktop-buffer {
    padding-left: 200px; /* Adjust as needed */
    padding-right: 100px; /* Adjust as needed */
  }
  .powered-by-container {
    padding-left: 150px;
    padding-right: 150px;
  }
  .intro-text {
    padding-bottom: 10px;
  }
}

.splash-image {
  width: 100%;
  height: auto;
  display: block; /* Remove extra space below the image */
}

.splash-image-smaller {
  max-width: 100%;
  height: auto;
  display: block;
  margin: 0 auto;
}

@media (min-width: 768px) {
  .splash-image-smaller {
    height: 500px;
    width: auto;
  }
}

.splash-padding-top {
  padding-top: 40px;
}


@media (max-width: 768px) {
  .intro-text {
    margin-bottom: 20px; /* Ensure spacing between the text and image on mobile */
  }
}

.intro-buttons {
  margin-top: 20px;
  margin-right: 100px;
  //spacing: 20px;
}
.intro-button {
  //margin-top: 20px;
  margin-right: 10px;
  //spacing: 20px;
  min-width: 135px;
  max-height: 40px;
}

.intro-text {
  //padding-bottom: 5%;
  //font-size: 1.2em;
  //font-weight: bold;
}

.intro-text h2 {
  font-size: 1.9em;
  font-weight: bold;
  font-family: 'Impact', sans-serif;
  color: #1dc4de;
  //margin-top: 10px;
}

.sub-intro-text {
  padding-top: 20px;
  font-size: 1.2em;
  //background-color: #2e2e2e
}

.sub-intro-text-section2 {
  padding-top: 20px;
  font-size: 1.2em;
  //background-color: #2e2e2e
}

.second-element {
  background-color: #121111;

}

.container-fluid {
  margin-top: 10px;
}



</style>


<script>

export default {
  name: 'HomePage',
  data() {
    return {};
  },
}
</script>

